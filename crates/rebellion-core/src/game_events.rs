//! Structured game event records for play-test telemetry.
//!
//! `GameEventRecord` is a pure data type — no IO. The caller
//! (rebellion-playtest or rebellion-app) handles serialization to JSONL.

use serde::{Deserialize, Serialize};

/// A single structured game event for play-test telemetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEventRecord {
    /// Game tick when this event occurred.
    pub tick: u64,
    /// Wall-clock milliseconds since session start.
    pub wall_ms: u64,
    /// Which simulation system emitted this event.
    pub system: &'static str,
    /// Event type within the system.
    pub event_type: &'static str,
    /// System-specific payload.
    pub details: serde_json::Value,
}

impl GameEventRecord {
    #[must_use]
    pub fn new(
        tick: u64,
        wall_ms: u64,
        system: &'static str,
        event_type: &'static str,
        details: serde_json::Value,
    ) -> Self {
        Self {
            tick,
            wall_ms,
            system,
            event_type,
            details,
        }
    }
}

// ---------------------------------------------------------------------------
// System name constants
// ---------------------------------------------------------------------------

pub const SYS_MANUFACTURING: &str = "manufacturing";
pub const SYS_MOVEMENT: &str = "movement";
pub const SYS_COMBAT: &str = "combat";
pub const SYS_FOG: &str = "fog";
pub const SYS_MISSIONS: &str = "missions";
pub const SYS_EVENTS: &str = "events";
pub const SYS_AI: &str = "ai";
pub const SYS_BLOCKADE: &str = "blockade";
pub const SYS_UPRISING: &str = "uprising";
pub const SYS_DEATH_STAR: &str = "death_star";
pub const SYS_RESEARCH: &str = "research";
pub const SYS_JEDI: &str = "jedi";
pub const SYS_VICTORY: &str = "victory";
pub const SYS_STORY: &str = "story";
pub const SYS_ECONOMY: &str = "economy";
pub const SYS_REPAIR: &str = "repair";

// ---------------------------------------------------------------------------
// Event type constants
// ---------------------------------------------------------------------------

pub const EVT_BUILD_COMPLETE: &str = "manufacturing_complete";
pub const EVT_FLEET_ARRIVED: &str = "fleet_arrived";
pub const EVT_FLEET_DEPARTED: &str = "fleet_departed";
pub const EVT_COMBAT_SPACE: &str = "combat_space";
pub const EVT_COMBAT_GROUND: &str = "combat_ground";
pub const EVT_BOMBARDMENT: &str = "bombardment";
pub const EVT_MISSION_DISPATCHED: &str = "mission_dispatched";
pub const EVT_MISSION_RESOLVED: &str = "mission_resolved";
/// Members of an ended mission were sent home on a Return order
/// (`FUN_00548910` -> `FUN_005475f0`).
pub const EVT_RETURN_ORDERED: &str = "return_ordered";
/// Adrift characters left on an Adrift order (`FUN_00549230` ->
/// `FUN_00547880`).
pub const EVT_ADRIFT_ORDERED: &str = "adrift_ordered";
pub const EVT_EVENT_FIRED: &str = "event_fired";
pub const EVT_AI_ACTION: &str = "ai_action";
pub const EVT_BLOCKADE_STARTED: &str = "blockade_started";
pub const EVT_BLOCKADE_ENDED: &str = "blockade_ended";
/// A regiment was destroyed leaving a blockaded system (event `0x340`).
pub const EVT_BLOCKADE_TROOP_DESTROYED: &str = "blockade_troop_destroyed";
/// Timer `0x386` shifted the blockading side's support (`FUN_0050cb20`).
pub const EVT_BLOCKADE_SUPPORT: &str = "blockade_support";
pub const EVT_UPRISING_INCIDENT: &str = "uprising_incident";
pub const EVT_UPRISING_BEGAN: &str = "uprising_began";
/// A revolt ended (`FUN_0050b800` or `FUN_0050c910`).
pub const EVT_UPRISING_ENDED: &str = "uprising_ended";
/// Timer `0x384` eroded the holder's support in revolt (`FUN_0050caa0`).
pub const EVT_UPRISING_SUPPORT: &str = "uprising_support";
pub const EVT_DS_CONSTRUCTION: &str = "death_star_construction";
pub const EVT_DS_FIRED: &str = "death_star_fired";
pub const EVT_RESEARCH_UNLOCKED: &str = "research_unlocked";
pub const EVT_VICTORY: &str = "victory_check";
pub const EVT_ESCAPE: &str = "character_escaped";
pub const EVT_STORY: &str = "story_event";
pub const EVT_CAPTURE: &str = "character_captured";
pub const EVT_FOG_REVEALED: &str = "fog_revealed";
pub const EVT_CONTROL_CHANGED: &str = "control_changed";
pub const EVT_CAMPAIGN_SNAPSHOT: &str = "campaign_snapshot";
pub const EVT_ECONOMY_TICK: &str = "economy_tick";
pub const EVT_SMUGGLING_PERCENT: &str = "smuggling_percent";
pub const EVT_COLLECTION_RATE: &str = "collection_rate";
pub const EVT_GARRISON_REQUIRED: &str = "garrison_required";
pub const EVT_SHIP_REPAIRED: &str = "ship_repaired";
pub const EVT_SHIP_REPAIR_STARTED: &str = "ship_repair_started";
pub const EVT_DECOY_TRIGGERED: &str = "decoy_triggered";
pub const EVT_DS_SHIELD_STATUS: &str = "death_star_shield_status";
pub const EVT_CHARACTER_COMBAT_RATING: &str = "character_combat_rating";
pub const EVT_STORY_JABBA: &str = "story_jabba";
pub const EVT_STORY_SEAT_OF_POWER: &str = "story_seat_of_power";
pub const EVT_VICTORY_CONDITIONS: &str = "victory_conditions";
pub const EVT_STORY_FINAL_BATTLE: &str = "story_final_battle";
pub const EVT_MAINTENANCE_SHORTFALL: &str = "maintenance_shortfall";

// System heartbeat events — emitted each tick to ensure system tags appear in telemetry
pub const EVT_UPRISING_CHECK: &str = "uprising_check";
pub const EVT_DS_STATUS: &str = "death_star_status";
pub const EVT_VICTORY_CHECK: &str = "victory_status";
/// A galaxy event's shift of every character's base loyalty
/// (`FUN_00561910`, `rebellion_core::loyalty`).
pub const EVT_LOYALTY_SHIFT: &str = "loyalty_shift";

// Phase 3 telemetry constants
pub const EVT_SUPPORT_CHANGE: &str = "support_change";
pub const EVT_UNITS_DEPLOYED: &str = "units_deployed";
/// A manufactured object lost because its destination no longer exists
/// (`GameObjDestroyedOnArrivalNotif`, event `0x303`).
pub const EVT_DESTROYED_ON_ARRIVAL: &str = "destroyed_on_arrival";
pub const EVT_HQ_CAPTURED: &str = "hq_captured";
pub const EVT_INFORMANT_INTEL: &str = "informant_intel";
pub const EVT_RESOURCE_DISCOVERY: &str = "resource_discovery";
pub const EVT_NATURAL_DISASTER: &str = "natural_disaster";
pub const EVT_MANUFACTURING_IDLE: &str = "manufacturing_idle";
pub const EVT_SABOTEUR_DETECTED: &str = "saboteur_detected";
pub const EVT_CHARACTER_KILLED: &str = "character_killed";
pub const EVT_CHARACTER_HEALTH: &str = "character_health";
pub const EVT_TRAITOR_REVEALED: &str = "traitor_revealed";
/// A trainer's timer `0x389` made a character Force aware (`FUN_00560a30`).
pub const EVT_FORCE_DISCOVERED: &str = "force_discovered";
/// A character's `ForceExperience` (`+0x8e`) grew (Luke's or Leia's evade,
/// `FUN_0056f8b0`, `FUN_0056ffa0`).
pub const EVT_FORCE_EXPERIENCE: &str = "force_experience";
/// `LukeDagobah` pulsed (notification `0x221`, `FUN_00544030`).
pub const EVT_LUKE_DAGOBAH: &str = "luke_dagobah";
/// Force detection paired two Force users (`FUN_0055ff60`, `Encounter`
/// `+0x9c`).
pub const EVT_FORCE_ENCOUNTER: &str = "force_encounter";
/// Two encountered Force users confronted each other (`FUN_00560200`).
pub const EVT_CONFRONTATION: &str = "confrontation";
/// Luke learned his heritage from Vader (`FUN_0056f650`).
pub const EVT_HERITAGE_LEARNED: &str = "heritage_learned";
pub const EVT_EMPEROR_ARRIVAL: &str = "emperor_arrival";
pub const EVT_JABBA_PRISONERS: &str = "jabba_prisoners";
pub const EVT_HAN_RESCUE: &str = "han_rescue";
pub const EVT_TROOP_MOVED: &str = "troop_moved";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_serializes_to_json() {
        let record = GameEventRecord::new(
            42,
            1200,
            SYS_MANUFACTURING,
            EVT_BUILD_COMPLETE,
            serde_json::json!({"system": "Coruscant"}),
        );
        let json = serde_json::to_string(&record).expect("serialize");
        assert!(json.contains("\"tick\":42"));
        assert!(json.contains("\"system\":\"manufacturing\""));
        assert!(json.contains("\"event_type\":\"manufacturing_complete\""));
    }

    #[test]
    fn record_deserializes_from_json() {
        let json = r#"{
            "tick": 10,
            "wall_ms": 500,
            "system": "combat",
            "event_type": "combat_space",
            "details": {"winner": "attacker"}
        }"#;
        let record: GameEventRecord = serde_json::from_str(json).expect("deserialize");
        assert_eq!(record.tick, 10);
        assert_eq!(record.wall_ms, 500);
        assert_eq!(record.system, "combat");
        assert_eq!(record.event_type, "combat_space");
    }

    #[test]
    fn all_event_type_constants_are_unique() {
        let all = [
            EVT_BUILD_COMPLETE,
            EVT_FLEET_ARRIVED,
            EVT_FLEET_DEPARTED,
            EVT_COMBAT_SPACE,
            EVT_COMBAT_GROUND,
            EVT_BOMBARDMENT,
            EVT_MISSION_DISPATCHED,
            EVT_MISSION_RESOLVED,
            EVT_RETURN_ORDERED,
            EVT_ADRIFT_ORDERED,
            EVT_EVENT_FIRED,
            EVT_AI_ACTION,
            EVT_BLOCKADE_STARTED,
            EVT_BLOCKADE_ENDED,
            EVT_BLOCKADE_TROOP_DESTROYED,
            EVT_BLOCKADE_SUPPORT,
            EVT_UPRISING_INCIDENT,
            EVT_UPRISING_BEGAN,
            EVT_UPRISING_ENDED,
            EVT_UPRISING_SUPPORT,
            EVT_DS_CONSTRUCTION,
            EVT_DS_FIRED,
            EVT_RESEARCH_UNLOCKED,
            EVT_VICTORY,
            EVT_ESCAPE,
            EVT_STORY,
            EVT_CAPTURE,
            EVT_FOG_REVEALED,
            EVT_CONTROL_CHANGED,
            EVT_CAMPAIGN_SNAPSHOT,
            EVT_ECONOMY_TICK,
            EVT_SMUGGLING_PERCENT,
            EVT_COLLECTION_RATE,
            EVT_GARRISON_REQUIRED,
            EVT_SHIP_REPAIRED,
            EVT_SHIP_REPAIR_STARTED,
            EVT_DECOY_TRIGGERED,
            EVT_DS_SHIELD_STATUS,
            EVT_CHARACTER_COMBAT_RATING,
            EVT_STORY_JABBA,
            EVT_STORY_SEAT_OF_POWER,
            EVT_VICTORY_CONDITIONS,
            EVT_STORY_FINAL_BATTLE,
            EVT_MAINTENANCE_SHORTFALL,
            // Phase 3
            EVT_SUPPORT_CHANGE,
            EVT_UNITS_DEPLOYED,
            EVT_HQ_CAPTURED,
            EVT_INFORMANT_INTEL,
            EVT_RESOURCE_DISCOVERY,
            EVT_NATURAL_DISASTER,
            EVT_MANUFACTURING_IDLE,
            EVT_SABOTEUR_DETECTED,
            EVT_CHARACTER_KILLED,
            EVT_CHARACTER_HEALTH,
            EVT_TRAITOR_REVEALED,
            EVT_FORCE_DISCOVERED,
            EVT_FORCE_EXPERIENCE,
            EVT_LUKE_DAGOBAH,
            EVT_FORCE_ENCOUNTER,
            EVT_CONFRONTATION,
            EVT_HERITAGE_LEARNED,
            EVT_EMPEROR_ARRIVAL,
            EVT_JABBA_PRISONERS,
            EVT_HAN_RESCUE,
            EVT_TROOP_MOVED,
            // Heartbeat events
            EVT_UPRISING_CHECK,
            EVT_DS_STATUS,
            EVT_VICTORY_CHECK,
            EVT_LOYALTY_SHIFT,
        ];
        let mut seen = std::collections::HashSet::new();
        for evt in &all {
            assert!(seen.insert(*evt), "duplicate event type constant: {evt}");
        }
    }
}
