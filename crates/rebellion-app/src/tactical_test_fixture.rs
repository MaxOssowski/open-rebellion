//! Test-only, versioned direct entry into the production tactical battle scene.

use rebellion_core::ids::{CapitalShipKey, FighterKey, SystemKey};
use rebellion_core::missions::MissionFaction;
use rebellion_core::troop_transport::TroopTransportState;
use rebellion_core::world::{FighterEntry, Fleet, GameWorld, ShipInstance};
use rebellion_render::tactical_view::{
    tactical_character_content_resource, tactical_content_icon, TacticalAttackTarget,
    TacticalUnitRef, WeaponKind,
};
#[cfg(feature = "interface-test-fixtures")]
use rebellion_render::TacticalLodView;
use rebellion_render::{
    CockpitFaction, CockpitState, MessageLog, TacticalState, TacticalTrenchRunOutcome,
};
use serde::Serialize;

use crate::tactical_flow::{self, BattleEntry};
use crate::GameMode;

const TACTICAL_FAMILY: u32 = 1;
const BATTLE_SCENARIO: u32 = 1;
const BATTLE_WITHOUT_PROOF_SCENARIO: u32 = 2;
const LOD_CLOSE_SCENARIO: u32 = 3;
const LOD_MEDIUM_SCENARIO: u32 = 4;
const LOD_FAR_SCENARIO: u32 = 5;
const LOD_JOURNEY_SCENARIO: u32 = 6;
const CAMERA_JOURNEY_SCENARIO: u32 = 7;
const PRODUCTION_PARTICIPANTS_SCENARIO: u32 = 8;
const PRODUCTION_PARTICIPANTS_CONTROL_SCENARIO: u32 = 9;
const PRODUCTION_FIGHTER_DETAIL_JOURNEY_SCENARIO: u32 = 10;
const GROUP_PRESENTATION_SCENARIO: u32 = 11;
const EFFECT_PRESENTATION_SCENARIO: u32 = 12;
const PROJECTILE_FIELD_PRESENTATION_SCENARIO: u32 = 13;
const SELECTED_DAMAGE_PRESENTATION_SCENARIO: u32 = 14;
const SUBSYSTEM_FIELD_COMMAND_PRESENTATION_SCENARIO: u32 = 15;
const LIVE_SUBSYSTEM_DAMAGE_PRESENTATION_SCENARIO: u32 = 16;
const SUBSYSTEM_REPAIR_MOBILITY_PRESENTATION_SCENARIO: u32 = 17;
const MANEUVER_MOVEMENT_PRESENTATION_SCENARIO: u32 = 18;
const COMMAND_ASSIGNMENT_PRESENTATION_SCENARIO: u32 = 19;
const COMMAND_EXECUTION_PRESENTATION_SCENARIO: u32 = 20;
const COMMAND_PROGRESSION_PRESENTATION_SCENARIO: u32 = 21;
const ATTACK_TARGETING_PRESENTATION_SCENARIO: u32 = 22;
const ATTACK_TARGET_LIFECYCLE_PRESENTATION_SCENARIO: u32 = 23;
const DEATH_STAR_PRESENTATION_SCENARIO: u32 = 24;
const BATTLE_RESULTS_PRESENTATION_SCENARIO: u32 = 25;
const BATTLE_OPTIONS_PRESENTATION_SCENARIO: u32 = 26;
const BATTLE_OPTIONS_WITHDRAWAL_SCENARIO: u32 = 27;
const DEATH_STAR_LASER_JOURNEY_SCENARIO: u32 = 28;
const TRENCH_RUN_SUCCESS_SCENARIO: u32 = 29;
const TRENCH_RUN_FAILURE_SCENARIO: u32 = 30;
const SELECTED_NAVIGATION_PRESENTATION_SCENARIO: u32 = 31;
const NAVIGATION_CAMERA_PRESENTATION_SCENARIO: u32 = 32;
const EMPTY_SPACE_PRESENTATION_SCENARIO: u32 = 33;
const DETAIL_ESCORT_PRESENTATION_SCENARIO: u32 = 34;
const BATTLE_ALERT_PRESENTATION_SCENARIO: u32 = 35;
const TACTICAL_AUDIO_PRESENTATION_SCENARIO: u32 = 36;
/// A0 TAC-01-C003's battle: one Medium Transport against one Victory
/// Destroyer, no fighters, so the extent is 103 (`FUN_005ab650`).
const HULL_SCALE_PRESENTATION_SCENARIO: u32 = 37;
/// A0 TAC-03-C017: the lifecycle fixture's battle after every hostile
/// capital has jumped out, so Attack Capital Ships holds no target.
const ATTACK_TARGET_LOST_PRESENTATION_SCENARIO: u32 = 38;
/// A0 message-rail/retreat-result-commenor and battle-alert-empire/04:
/// Retreat on the Battle Alert at the viewer's own system, which the enemy
/// reached last, by a fleet weak enough to leave at once (FUN_005d19a0).
const BATTLE_ALERT_RETREAT_SCENARIO: u32 = 39;
/// A0 TAC-07-C004: the viewer's one hull withdraws from a played battle at
/// the enemy's system (Withdraw From Battle, FUN_005f1170) until the
/// battle ends and the report reads its failed attack and withdrawal.
const BATTLE_WITHDRAWAL_REPORT_SCENARIO: u32 = 40;
/// A0 TAC-01-C013 and TAC-07-C014: A0 TAC-01-C003's battle at Imperial
/// Mon Calamari played until the Alliance's last hull dies, then the
/// Battle Summary's system button (`0xcb`, `FUN_00470ea0`).
const BATTLE_LAST_SHIP_LOST_SCENARIO: u32 = 41;
/// A0 TAC-01-C008: a battle at Umgul, whose planet the camera shows at the
/// holocube's top right after Rotate Left 16 times.
const UMGUL_BACKDROP_SCENARIO: u32 = 42;
/// A0 TAC-04-C014: the Alliance's Battle at Imperial Coruscant (manifest
/// `new-game-alliance-small-2026-10-24-coruscant2`): Fleet 2, an Alliance
/// Escort Carrier with one X-wing and one Y-wing squadron, against the
/// home fleet the galaxy places at Coruscant.
const CORUSCANT_CHASE_SCENARIO: u32 = 43;
/// The last-ship battle with MDATA.104 (Death Star Sabotaged) queued: it
/// plays once the battle ends, over the open Battle Summary (the queue
/// drains while the clock is not held: `FUN_00488b90`, `FUN_0041dc10`).
const BATTLE_SUMMARY_STORY_MOVIE_SCENARIO: u32 = 44;
/// A0 TAC-01-C007 and TAC-01-C009's battle (manifest
/// `new-game-empire-small-2026-10-24-yavin`, day 234, Show Planet off):
/// Imperial Fleet 1, an Imperial Star Destroyer with six TIE Fighter
/// squadrons and a Victory Destroyer with one, against the Alliance's
/// Corellian Corvette Swift at Yavin.
const YAVIN_MULTI_SHIP_SCENARIO: u32 = 45;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TacticalLodFixture {
    Default,
    Close,
    Medium,
    Far,
    Journey,
    CameraJourney,
}

impl TacticalLodFixture {
    const fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Close => "close",
            Self::Medium => "medium",
            Self::Far => "far",
            Self::Journey => "journey",
            Self::CameraJourney => "camera-journey",
        }
    }

    #[cfg(feature = "interface-test-fixtures")]
    const fn view(self) -> Option<TacticalLodView> {
        match self {
            Self::Default => None,
            Self::Close => Some(TacticalLodView::CLOSE_FIXTURE),
            Self::Medium => Some(TacticalLodView::MEDIUM_FIXTURE),
            Self::Far => Some(TacticalLodView::FAR_FIXTURE),
            Self::Journey => None,
            Self::CameraJourney => None,
        }
    }
}

extern "C" {
    fn open_rebellion_interface_fixture_code() -> u32;
    fn open_rebellion_interface_fixture_emit(ptr: *const u8, len: usize);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TacticalFixtureRequest {
    pub faction: CockpitFaction,
    pub code: u32,
    pub proof_enabled: bool,
    pub production_participants: bool,
    pub suppress_capital_fallback: bool,
    pub focus_player_fighter: bool,
    pub group_presentation: bool,
    pub effect_presentation: bool,
    pub projectile_field_presentation: bool,
    pub selected_damage_presentation: bool,
    pub selected_navigation_presentation: bool,
    pub navigation_camera_presentation: bool,
    pub empty_space_presentation: bool,
    pub detail_escort_presentation: bool,
    pub battle_alert_presentation: bool,
    pub tactical_audio_presentation: bool,
    pub hull_scale_presentation: bool,
    pub attack_target_lost_presentation: bool,
    pub battle_alert_retreat: bool,
    pub battle_withdrawal_report: bool,
    pub battle_last_ship_lost: bool,
    pub summary_story_movie: bool,
    pub umgul_backdrop: bool,
    pub coruscant_chase: bool,
    pub yavin_multi_ship: bool,
    pub subsystem_field_command_presentation: bool,
    pub live_subsystem_damage_presentation: bool,
    pub subsystem_repair_mobility_presentation: bool,
    pub maneuver_movement_presentation: bool,
    pub command_assignment_presentation: bool,
    pub command_execution_presentation: bool,
    pub command_progression_presentation: bool,
    pub attack_targeting_presentation: bool,
    pub attack_target_lifecycle_presentation: bool,
    pub death_star_presentation: bool,
    pub battle_results_presentation: bool,
    pub battle_options_presentation: bool,
    pub battle_options_withdrawal: bool,
    pub death_star_laser_journey: bool,
    pub trench_run_outcome: Option<TacticalTrenchRunOutcome>,
    lod_fixture: TacticalLodFixture,
}

fn decode(code: u32) -> Option<TacticalFixtureRequest> {
    if code >> 16 != TACTICAL_FAMILY {
        return None;
    }
    let scenario = code & 0xff;
    let (
        proof_enabled,
        production_participants,
        suppress_capital_fallback,
        focus_player_fighter,
        group_presentation,
        effect_presentation,
        projectile_field_presentation,
        selected_damage_presentation,
        lod_fixture,
    ) = match scenario {
        BATTLE_SCENARIO => (
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            TacticalLodFixture::Default,
        ),
        BATTLE_WITHOUT_PROOF_SCENARIO => (
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            TacticalLodFixture::Default,
        ),
        LOD_CLOSE_SCENARIO => (
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            TacticalLodFixture::Close,
        ),
        LOD_MEDIUM_SCENARIO => (
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            TacticalLodFixture::Medium,
        ),
        LOD_FAR_SCENARIO => (
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            TacticalLodFixture::Far,
        ),
        LOD_JOURNEY_SCENARIO => (
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            TacticalLodFixture::Journey,
        ),
        CAMERA_JOURNEY_SCENARIO => (
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            TacticalLodFixture::CameraJourney,
        ),
        PRODUCTION_PARTICIPANTS_SCENARIO => (
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            TacticalLodFixture::Default,
        ),
        PRODUCTION_PARTICIPANTS_CONTROL_SCENARIO => (
            false,
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            TacticalLodFixture::Default,
        ),
        PRODUCTION_FIGHTER_DETAIL_JOURNEY_SCENARIO => (
            false,
            true,
            false,
            true,
            false,
            false,
            false,
            false,
            TacticalLodFixture::Default,
        ),
        GROUP_PRESENTATION_SCENARIO => (
            false,
            true,
            false,
            false,
            true,
            false,
            false,
            false,
            TacticalLodFixture::Default,
        ),
        EFFECT_PRESENTATION_SCENARIO => (
            false,
            true,
            false,
            false,
            false,
            true,
            false,
            false,
            TacticalLodFixture::Default,
        ),
        PROJECTILE_FIELD_PRESENTATION_SCENARIO => (
            false,
            true,
            false,
            false,
            false,
            false,
            true,
            false,
            TacticalLodFixture::Default,
        ),
        SELECTED_DAMAGE_PRESENTATION_SCENARIO => (
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            true,
            TacticalLodFixture::Default,
        ),
        SUBSYSTEM_FIELD_COMMAND_PRESENTATION_SCENARIO => (
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            TacticalLodFixture::Default,
        ),
        LIVE_SUBSYSTEM_DAMAGE_PRESENTATION_SCENARIO => (
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            TacticalLodFixture::Default,
        ),
        SUBSYSTEM_REPAIR_MOBILITY_PRESENTATION_SCENARIO
        | MANEUVER_MOVEMENT_PRESENTATION_SCENARIO
        | COMMAND_ASSIGNMENT_PRESENTATION_SCENARIO
        | COMMAND_EXECUTION_PRESENTATION_SCENARIO
        | COMMAND_PROGRESSION_PRESENTATION_SCENARIO
        | ATTACK_TARGETING_PRESENTATION_SCENARIO
        | ATTACK_TARGET_LIFECYCLE_PRESENTATION_SCENARIO
        | DEATH_STAR_PRESENTATION_SCENARIO
        | BATTLE_RESULTS_PRESENTATION_SCENARIO
        | BATTLE_OPTIONS_PRESENTATION_SCENARIO
        | BATTLE_OPTIONS_WITHDRAWAL_SCENARIO
        | DEATH_STAR_LASER_JOURNEY_SCENARIO
        | TRENCH_RUN_SUCCESS_SCENARIO
        | TRENCH_RUN_FAILURE_SCENARIO
        | SELECTED_NAVIGATION_PRESENTATION_SCENARIO
        | NAVIGATION_CAMERA_PRESENTATION_SCENARIO
        | EMPTY_SPACE_PRESENTATION_SCENARIO
        | DETAIL_ESCORT_PRESENTATION_SCENARIO
        | BATTLE_ALERT_PRESENTATION_SCENARIO
        | TACTICAL_AUDIO_PRESENTATION_SCENARIO
        | HULL_SCALE_PRESENTATION_SCENARIO
        | ATTACK_TARGET_LOST_PRESENTATION_SCENARIO
        | BATTLE_ALERT_RETREAT_SCENARIO
        | BATTLE_WITHDRAWAL_REPORT_SCENARIO
        | BATTLE_LAST_SHIP_LOST_SCENARIO
        | UMGUL_BACKDROP_SCENARIO
        | CORUSCANT_CHASE_SCENARIO
        | BATTLE_SUMMARY_STORY_MOVIE_SCENARIO
        | YAVIN_MULTI_SHIP_SCENARIO => (
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            TacticalLodFixture::Default,
        ),
        _ => return None,
    };
    let faction = match (code >> 8) & 0xff {
        1 => CockpitFaction::Alliance,
        2 => CockpitFaction::Empire,
        _ => return None,
    };
    Some(TacticalFixtureRequest {
        faction,
        code,
        proof_enabled,
        production_participants,
        suppress_capital_fallback,
        focus_player_fighter: focus_player_fighter
            || matches!(
                scenario,
                DEATH_STAR_PRESENTATION_SCENARIO
                    | DEATH_STAR_LASER_JOURNEY_SCENARIO
                    | TRENCH_RUN_SUCCESS_SCENARIO
                    | TRENCH_RUN_FAILURE_SCENARIO
            ),
        group_presentation,
        effect_presentation,
        projectile_field_presentation,
        selected_damage_presentation,
        selected_navigation_presentation: scenario == SELECTED_NAVIGATION_PRESENTATION_SCENARIO,
        navigation_camera_presentation: scenario == NAVIGATION_CAMERA_PRESENTATION_SCENARIO,
        empty_space_presentation: scenario == EMPTY_SPACE_PRESENTATION_SCENARIO,
        detail_escort_presentation: scenario == DETAIL_ESCORT_PRESENTATION_SCENARIO,
        battle_alert_presentation: scenario == BATTLE_ALERT_PRESENTATION_SCENARIO,
        tactical_audio_presentation: scenario == TACTICAL_AUDIO_PRESENTATION_SCENARIO,
        hull_scale_presentation: matches!(
            scenario,
            HULL_SCALE_PRESENTATION_SCENARIO
                | BATTLE_LAST_SHIP_LOST_SCENARIO
                | BATTLE_SUMMARY_STORY_MOVIE_SCENARIO
        ),
        attack_target_lost_presentation: scenario == ATTACK_TARGET_LOST_PRESENTATION_SCENARIO,
        battle_alert_retreat: scenario == BATTLE_ALERT_RETREAT_SCENARIO,
        battle_withdrawal_report: scenario == BATTLE_WITHDRAWAL_REPORT_SCENARIO,
        battle_last_ship_lost: matches!(
            scenario,
            BATTLE_LAST_SHIP_LOST_SCENARIO | BATTLE_SUMMARY_STORY_MOVIE_SCENARIO
        ),
        summary_story_movie: scenario == BATTLE_SUMMARY_STORY_MOVIE_SCENARIO,
        umgul_backdrop: scenario == UMGUL_BACKDROP_SCENARIO,
        coruscant_chase: scenario == CORUSCANT_CHASE_SCENARIO,
        yavin_multi_ship: scenario == YAVIN_MULTI_SHIP_SCENARIO,
        subsystem_field_command_presentation: scenario
            == SUBSYSTEM_FIELD_COMMAND_PRESENTATION_SCENARIO,
        live_subsystem_damage_presentation: scenario == LIVE_SUBSYSTEM_DAMAGE_PRESENTATION_SCENARIO,
        subsystem_repair_mobility_presentation: scenario
            == SUBSYSTEM_REPAIR_MOBILITY_PRESENTATION_SCENARIO,
        maneuver_movement_presentation: scenario == MANEUVER_MOVEMENT_PRESENTATION_SCENARIO,
        command_assignment_presentation: scenario == COMMAND_ASSIGNMENT_PRESENTATION_SCENARIO,
        command_execution_presentation: scenario == COMMAND_EXECUTION_PRESENTATION_SCENARIO,
        command_progression_presentation: scenario == COMMAND_PROGRESSION_PRESENTATION_SCENARIO,
        attack_targeting_presentation: scenario == ATTACK_TARGETING_PRESENTATION_SCENARIO,
        attack_target_lifecycle_presentation: matches!(
            scenario,
            ATTACK_TARGET_LIFECYCLE_PRESENTATION_SCENARIO | ATTACK_TARGET_LOST_PRESENTATION_SCENARIO
        ),
        death_star_presentation: matches!(
            scenario,
            DEATH_STAR_PRESENTATION_SCENARIO
                | DEATH_STAR_LASER_JOURNEY_SCENARIO
                | TRENCH_RUN_SUCCESS_SCENARIO
                | TRENCH_RUN_FAILURE_SCENARIO
        ),
        battle_results_presentation: scenario == BATTLE_RESULTS_PRESENTATION_SCENARIO,
        battle_options_presentation: matches!(
            scenario,
            BATTLE_OPTIONS_PRESENTATION_SCENARIO | BATTLE_OPTIONS_WITHDRAWAL_SCENARIO
        ),
        battle_options_withdrawal: scenario == BATTLE_OPTIONS_WITHDRAWAL_SCENARIO,
        death_star_laser_journey: scenario == DEATH_STAR_LASER_JOURNEY_SCENARIO,
        trench_run_outcome: match scenario {
            TRENCH_RUN_SUCCESS_SCENARIO => Some(TacticalTrenchRunOutcome::Success),
            TRENCH_RUN_FAILURE_SCENARIO => Some(TacticalTrenchRunOutcome::Failure),
            _ => None,
        },
        lod_fixture,
    })
}

pub(crate) fn requested() -> Option<TacticalFixtureRequest> {
    decode(unsafe { open_rebellion_interface_fixture_code() })
}

/// Add a minimal opposing encounter and enter it through the same function as
/// campaign-triggered battles. This function is not compiled into production.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep every mutated campaign state explicit at this test-only bridge."
)]
pub(crate) fn apply(
    request: TacticalFixtureRequest,
    world: &mut GameWorld,
    troop_transport: &mut TroopTransportState,
    tactical: &mut TacticalState,
    cooldowns: &mut std::collections::HashMap<SystemKey, u64>,
    messages: &mut MessageLog,
    player_faction: &mut MissionFaction,
    cockpit: &mut CockpitState,
    game_mode: &mut GameMode,
) -> Result<Vec<rebellion_core::squadrons::Sortie>, String> {
    // Exercise two real SYSTEMSD picture/palette identities across the faction
    // matrix instead of accepting a hard-coded global palette accidentally:
    // the first and second empty, intact systems the galaxy loaded, in
    // SYSTEMSD order. The galaxy size picks which records load (the default
    // loads indices 120..289), so no fixed index is assumed.
    let mut empty: Vec<(u32, SystemKey)> = world
        .systems
        .iter()
        .filter(|(_, system)| !system.is_destroyed && system.fleets.is_empty())
        .map(|(key, system)| (system.dat_id.index(), key))
        .collect();
    empty.sort_unstable();
    let ordinal = usize::from(request.faction != CockpitFaction::Alliance);
    let system = if request.coruscant_chase {
        // A0 TAC-04-C014 was taken at Coruscant, whose home fleet defends:
        // the fixture keeps the fleets the galaxy placed there.
        world
            .systems
            .iter()
            .find(|(_, system)| !system.is_destroyed && system.name == "Coruscant")
            .map(|(key, _)| key)
            .ok_or("fixture system Coruscant is missing")?
    } else if request.yavin_multi_ship {
        // A0 TAC-01-C007 was taken at Yavin. Its own fleets stay out.
        let key = world
            .systems
            .iter()
            .find(|(_, system)| !system.is_destroyed && system.name == "Yavin")
            .map(|(key, _)| key)
            .ok_or("fixture system Yavin is missing")?;
        world.systems[key].fleets.clear();
        key
    } else if request.umgul_backdrop {
        // A0 TAC-01-C008 was taken at Umgul: the same system gives the
        // same planet decal. Its own fleets stay out.
        let key = world
            .systems
            .iter()
            .find(|(_, system)| !system.is_destroyed && system.name == "Umgul")
            .map(|(key, _)| key)
            .ok_or("fixture system Umgul is missing")?;
        world.systems[key].fleets.clear();
        key
    } else if request.hull_scale_presentation {
        // A0 TAC-01-C003 was taken at Mon Calamari: the same system gives
        // the same planet and battle palette. Its own fleets stay out.
        let key = world
            .systems
            .iter()
            .find(|(_, system)| !system.is_destroyed && system.name == "Mon Calamari")
            .map(|(key, _)| key)
            .ok_or("fixture system Mon Calamari is missing")?;
        world.systems[key].fleets.clear();
        key
    } else {
        empty
            .get(ordinal)
            .map(|&(_, key)| key)
            .ok_or("fixture system is not empty and intact")?
    };
    let alliance_ship = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| {
            class.is_alliance
                && !class.is_empire
                && class.hull > 0
                && (!request.hull_scale_presentation || class.name.contains("Medium Transport"))
                && (!request.detail_escort_presentation || class.troop_capacity >= 3)
                && (!(request.battle_options_presentation
                    || request.battle_alert_retreat
                    || request.battle_withdrawal_report)
                    || class.hyperdrive > 0)
                && (!(request.command_execution_presentation
                    || request.command_progression_presentation
                    || request.attack_targeting_presentation
                    || request.attack_target_lifecycle_presentation)
                    || class.fighter_capacity > 0)
        })
        .map(|(key, class)| (key, class.hull))
        .ok_or("no Alliance capital-ship class")?;
    let empire_ship = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| {
            class.is_empire
                && !class.is_alliance
                && class.hull > 0
                && (!request.hull_scale_presentation || class.name.contains("Victory"))
                && (!request.detail_escort_presentation || class.troop_capacity >= 3)
                && (!(request.battle_options_presentation
                    || request.battle_alert_retreat
                    || request.battle_withdrawal_report)
                    || class.hyperdrive > 0)
                && (!(request.command_execution_presentation
                    || request.command_progression_presentation
                    || request.attack_targeting_presentation
                    || request.attack_target_lifecycle_presentation)
                    || class.fighter_capacity > 0)
        })
        .map(|(key, class)| (key, class.hull))
        .ok_or("no Imperial capital-ship class")?;
    let alliance_fighter = world
        .fighter_classes
        .iter()
        .find(|(_, class)| class.is_alliance && !class.is_empire)
        .map(|(key, _)| key);
    let empire_fighter = world
        .fighter_classes
        .iter()
        .find(|(_, class)| class.is_empire && !class.is_alliance)
        .map(|(key, _)| key);

    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let make_fleet = |is_alliance: bool,
                      ship: CapitalShipKey,
                      hull: u32,
                      fighter: Option<FighterKey>| {
        let expanded = request.projectile_field_presentation
            || request.detail_escort_presentation
            || request.subsystem_field_command_presentation
            || request.live_subsystem_damage_presentation
            || request.subsystem_repair_mobility_presentation
            || request.maneuver_movement_presentation
            || request.command_execution_presentation
            || request.command_progression_presentation
            || request.attack_targeting_presentation
            || request.attack_target_lifecycle_presentation
            || request.battle_results_presentation
            || request.selected_navigation_presentation
            || request.navigation_camera_presentation
            || ((request.group_presentation || request.effect_presentation)
                && is_alliance == player_is_alliance);
        let ship_count = if request.battle_alert_retreat {
            // One hull against four: the viewer's strength is a quarter
            // of the enemy's, under the leave threshold 0.33 (+0x24,
            // FUN_005c7cb0), so it leaves at the first step.
            if is_alliance == player_is_alliance {
                1
            } else {
                4
            }
        } else if request.battle_options_withdrawal {
            2
        } else if request.projectile_field_presentation
            || request.detail_escort_presentation
            || request.subsystem_field_command_presentation
            || request.live_subsystem_damage_presentation
            || request.subsystem_repair_mobility_presentation
            || request.maneuver_movement_presentation
            || request.command_execution_presentation
            || request.command_progression_presentation
            || request.attack_targeting_presentation
            || request.attack_target_lifecycle_presentation
            || request.battle_results_presentation
            || request.selected_navigation_presentation
            || request.navigation_camera_presentation
        {
            3
        } else if request.effect_presentation && expanded {
            6
        } else if expanded {
            8
        } else {
            1
        };
        let fighter_count = if request.battle_results_presentation || request.battle_alert_presentation {
            // Four squadrons stay operational: the Fighters page's list
            // then outgrows its 166 pixels and shows its scroll bar.
            5
        } else if request.command_assignment_presentation
            || request.command_progression_presentation
            || request.attack_targeting_presentation
            || request.attack_target_lifecycle_presentation
        {
            2
        } else if request.group_presentation && expanded {
            4
        } else if request.hull_scale_presentation
            || request.battle_alert_retreat
            || request.battle_withdrawal_report
        {
            0
        } else if request.death_star_presentation && !is_alliance {
            // FUN_005ad930 enables Attack Death Star only while fewer
            // Imperial than Alliance fighter groups are on the field.
            0
        } else {
            1
        };
        Fleet {
            location: system,
            capital_ships: (0..ship_count)
                .map(|_| {
                    ShipInstance::new(ship, i32::try_from(hull).unwrap_or(i32::MAX), is_alliance)
                })
                .collect(),
            fighters: fighter
                .map(|class| {
                    (0..fighter_count)
                        // `FighterEntry::count` is the number of strategic
                        // squadrons. `FIGHTSD.squadron_size` supplies the
                        // twelve craft inside each tactical object.
                        .map(|_| FighterEntry { class, count: 1, carrier: None })
                        .collect()
                })
                .unwrap_or_default(),
            characters: vec![],
            is_alliance,
            has_death_star: request.death_star_presentation && !is_alliance,
            special_forces: Vec::new(),
        }
    };
    let (attacker, defender) = if request.coruscant_chase {
        coruscant_chase_fleets(world, system)?
    } else if request.yavin_multi_ship {
        yavin_multi_ship_fleets(world, system)?
    } else {
        (
            world.fleets.insert(make_fleet(
                true,
                alliance_ship.0,
                alliance_ship.1,
                alliance_fighter,
            )),
            world.fleets.insert(make_fleet(
                false,
                empire_ship.0,
                empire_ship.1,
                empire_fighter,
            )),
        )
    };
    if request.detail_escort_presentation {
        let player_fleet = if player_is_alliance {
            attacker
        } else {
            defender
        };
        let character = world
            .characters
            .iter()
            .find(|(_, character)| {
                !character.is_killed
                    && character.is_major
                    && character.current_fleet.is_none()
                    && tactical_character_content_resource(character.dat_id, character.is_major)
                        .is_some()
                    && if player_is_alliance {
                        character.is_alliance
                    } else {
                        character.is_empire
                    }
            })
            .map(|(key, _)| key)
            .ok_or("no unassigned faction character for tactical contents fixture")?;
        // Number the player's hulls as the game does (`last_ship_id`): the
        // character rides on the first numbered one (`ship_cargo`).
        for index in 0..world.fleets[player_fleet].capital_ships.len() {
            world.last_ship_id += 1;
            world.fleets[player_fleet].capital_ships[index].id = world.last_ship_id;
        }
        world.fleets[player_fleet].characters.push(character);
        world.characters[character].current_fleet = Some(player_fleet);
        world.characters[character].current_system = None;
        // Three of the side's regiments aboard, as A0 TAC-03-C012's
        // transport carries Alliance Army Regiments and the Empire's
        // Coruscant battle (a0-vm/coruscant-676/05) a character and three
        // Stormtrooper Regiments, which overflow the list into its bar.
        let class = rebellion_core::ids::DatId::new(if player_is_alliance {
            0x1000_0002
        } else {
            0x1000_0006
        });
        let troops = [0, 1, 2].map(|_| {
            world.troops.insert(rebellion_core::world::TroopUnit {
                class_dat_id: class,
                is_alliance: player_is_alliance,
                regiment_strength: 1,
            })
        });
        world.systems[system].ground_units.extend(troops);
        troop_transport
            .embark(world, player_fleet, &troops)
            .map_err(|error| format!("contents fixture regiments: {error}"))?;
    }
    // The Alliance's results carry three Alliance Army Regiments on its
    // three one-regiment hulls (CAPSHPSD 64, troop capacity 1); the third
    // hull dies (configure_battle_results_presentation_fixture), so one
    // regiment is listed destroyed, as A0 TAC-07-C007's Troops page lists
    // its Alliance Army Regiments.
    if request.battle_results_presentation && player_is_alliance {
        let troops = [0, 1, 2].map(|_| {
            world.troops.insert(rebellion_core::world::TroopUnit {
                class_dat_id: rebellion_core::ids::DatId::new(0x1000_0002),
                is_alliance: true,
                regiment_strength: 1,
            })
        });
        world.systems[system].ground_units.extend(troops);
        troop_transport
            .embark(world, attacker, &troops)
            .map_err(|error| format!("results fixture regiments: {error}"))?;
    }
    if request.coruscant_chase {
        // The home fleet is already listed at Coruscant. The Alliance
        // arrived last at the Empire's system (A0: Fleet 2 dragged from
        // Averam onto Imperial Coruscant).
        world.systems[system].fleets.push(attacker);
        world.systems[system].control =
            rebellion_core::world::ControlKind::Controlled(rebellion_core::dat::Faction::Empire);
        world.systems[system].last_arrival = Some(rebellion_core::dat::Faction::Alliance);
    } else {
        world.systems[system].fleets.extend([attacker, defender]);
    }
    // The Empire's alert system holds Tralus's assets as A0
    // battle-alert-empire/01-system-assets lists them: GenCore Level I,
    // Orbital Shipyard, Construction Yard and two mines, nothing else.
    #[cfg(feature = "interface-test-fixtures")]
    if request.battle_alert_presentation && request.faction == CockpitFaction::Empire {
        use rebellion_core::dat::Faction;
        use rebellion_core::world::{
            DefenseFacilityInstance, ManufacturingFacilityInstance, ProductionFacilityInstance,
        };
        let gencore = world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id: rebellion_core::ids::DatId::new(0x2300_0003),
            side: Faction::Empire,
        });
        let yards = [(0x2800_0001, true), (0x2a00_0003, false)].map(|(id, is_shipyard)| {
            world.manufacturing_facilities.insert(ManufacturingFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(id),
                side: Faction::Empire,
                is_shipyard,
            })
        });
        let mines = [0, 1].map(|_| {
            world.production_facilities.insert(ProductionFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2c00_0001),
                side: Faction::Empire,
                is_mine: true,
            })
        });
        let place = &mut world.systems[system];
        place.defense_facilities = vec![gencore];
        place.manufacturing_facilities = yards.to_vec();
        place.production_facilities = mines.to_vec();
        place.ground_units.clear();
        place.special_forces.clear();
        for character in world.characters.values_mut() {
            if character.current_system == Some(system) {
                character.current_system = None;
            }
        }
    }

    if request.battle_alert_retreat {
        // The viewer's system, which the enemy reached last (+0x78 bits
        // 6..7): FUN_0049b4d0 then reads 0x7008 "now under blockade".
        use rebellion_core::dat::Faction;
        let (viewer, enemy) = if player_is_alliance {
            (Faction::Alliance, Faction::Empire)
        } else {
            (Faction::Empire, Faction::Alliance)
        };
        world.systems[system].control = rebellion_core::world::ControlKind::Controlled(viewer);
        world.systems[system].last_arrival = Some(enemy);
    }
    if request.battle_withdrawal_report {
        // The enemy's system, which the viewer reached last: FUN_0049b4d0
        // then reads 0x7009 "The {viewer} attack on {system} has failed."
        use rebellion_core::dat::Faction;
        let (viewer, enemy) = if player_is_alliance {
            (Faction::Alliance, Faction::Empire)
        } else {
            (Faction::Empire, Faction::Alliance)
        };
        world.systems[system].control = rebellion_core::world::ControlKind::Controlled(enemy);
        world.systems[system].last_arrival = Some(viewer);
    }
    if request.battle_last_ship_lost {
        // A0 TAC-01-C013: the Empire held Mon Calamari and its fleet
        // arrived last, against the Alliance's blockade; FUN_0049b4d0
        // then reads 0x7016 "Imperial forces have broken the blockade".
        use rebellion_core::dat::Faction;
        world.systems[system].control =
            rebellion_core::world::ControlKind::Controlled(Faction::Empire);
        world.systems[system].last_arrival = Some(Faction::Empire);
    }

    let player_is_attacker = request.faction == CockpitFaction::Alliance;
    // FUN_0040b0e0 (0x40b7d5) builds every battle with the system's planet
    // squadrons, as the campaign's entry does (squadrons::scramble); they
    // land again when it ends.
    let sorties = rebellion_core::squadrons::scramble(world, system);
    tactical_flow::begin_player_battle(
        world,
        troop_transport,
        BattleEntry {
            system,
            attacker,
            defender,
            player_is_attacker,
            tick: 0,
            rng_seed: 0x5eed_ba77,
        },
        tactical,
        cooldowns,
        messages,
        game_mode,
    )
    .map_err(|error| format!("fixture battle entry failed: {error:?}"))?;
    // The backdrop's framing follows the clock second (FUN_005f5750). A0
    // TAC-01-C008's planet is one of its draws; the other scenarios keep
    // both frames centred so their pictures do not change between runs.
    #[cfg(feature = "interface-test-fixtures")]
    tactical.set_backdrop_clock_for_fixture(
        request
            .umgul_backdrop
            .then_some(rebellion_render::UMGUL_A0_BACKDROP_CLOCK),
    );
    #[cfg(feature = "interface-test-fixtures")]
    if !(request.battle_alert_presentation || request.battle_alert_retreat) {
        tactical.dismiss_battle_alert_for_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.focus_player_fighter {
        tactical.focus_player_fighter_for_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.group_presentation {
        tactical.configure_group_presentation_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.effect_presentation {
        tactical.configure_effect_presentation_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.projectile_field_presentation {
        tactical.configure_projectile_field_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.selected_damage_presentation {
        tactical.configure_selected_damage_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.selected_navigation_presentation {
        tactical.configure_selected_navigation_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.navigation_camera_presentation {
        tactical.configure_selected_navigation_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.subsystem_field_command_presentation {
        tactical.configure_subsystem_field_command_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.live_subsystem_damage_presentation {
        tactical.configure_live_subsystem_damage_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.subsystem_repair_mobility_presentation {
        tactical.configure_subsystem_repair_mobility_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.maneuver_movement_presentation {
        tactical.configure_maneuver_movement_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.command_assignment_presentation {
        tactical.configure_command_assignment_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.command_execution_presentation {
        tactical.configure_command_execution_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.command_progression_presentation {
        tactical.configure_command_progression_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.attack_targeting_presentation {
        tactical.configure_attack_targeting_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.attack_target_lifecycle_presentation && !request.attack_target_lost_presentation {
        tactical.configure_attack_target_lifecycle_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.attack_target_lost_presentation {
        tactical.configure_attack_target_lost_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    // Outcome fixtures enter the same production Death Star path themselves.
    // Do not pre-arm the Imperial observer route first: that would consume the
    // fixture's deterministic result before its requested seed/rating is set.
    if request.death_star_presentation && request.trench_run_outcome.is_none() {
        tactical.configure_death_star_presentation_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.death_star_laser_journey {
        tactical.configure_death_star_laser_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if let Some(outcome) = request.trench_run_outcome {
        tactical.configure_trench_run_outcome_fixture(outcome);
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.battle_results_presentation {
        tactical.configure_battle_results_presentation_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.battle_options_withdrawal {
        tactical.configure_withdrawal_warning_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.battle_last_ship_lost {
        tactical.configure_last_alliance_ship_lost_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.summary_story_movie {
        // FUN_0048d470's caption (TEXTSTRA 0x71be) is not what this
        // fixture checks; the movie's music is.
        messages.queue_story_movie(rebellion_render::StoryMovie {
            movie: 104,
            caption: "The Rebel Alliance has sabotaged the Death Star at Endor.".to_owned(),
        });
    }
    // A0 TAC-01-C007's battle ran with Show Planet off.
    #[cfg(feature = "interface-test-fixtures")]
    if request.empty_space_presentation || request.yavin_multi_ship {
        tactical.configure_empty_space_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.detail_escort_presentation {
        tactical.configure_detail_escort_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.tactical_audio_presentation {
        tactical.configure_tactical_audio_fixture();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if !request.production_participants {
        tactical.disable_original_participant_rendering();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.suppress_capital_fallback {
        tactical.suppress_mapped_capital_fallback();
    }
    #[cfg(feature = "interface-test-fixtures")]
    if request.proof_enabled {
        tactical.enable_resource_2560_proof();
        if request.lod_fixture == TacticalLodFixture::Journey {
            tactical.enable_tactical_lod_journey();
        } else if request.lod_fixture == TacticalLodFixture::CameraJourney {
            tactical.enable_original_camera_proof(request.faction == CockpitFaction::Empire);
        } else if let Some(view) = request.lod_fixture.view() {
            tactical.set_tactical_lod_fixture(view);
        }
    }
    *player_faction = if player_is_attacker {
        MissionFaction::Alliance
    } else {
        MissionFaction::Empire
    };
    cockpit.faction = request.faction;
    Ok(sorties)
}

#[derive(Serialize)]
struct FixtureRecord<'a> {
    schema_version: u32,
    status: &'a str,
    family: &'a str,
    fixture_code: u32,
    faction: &'a str,
    proof_enabled: bool,
    production_participants: bool,
    suppress_capital_fallback: bool,
    focus_player_fighter: bool,
    group_presentation: bool,
    effect_presentation: bool,
    projectile_field_presentation: bool,
    selected_damage_presentation: bool,
    selected_navigation_presentation: bool,
    navigation_camera_presentation: bool,
    empty_space_presentation: bool,
    detail_escort_presentation: bool,
    battle_alert_presentation: bool,
    tactical_audio_presentation: bool,
    hull_scale_presentation: bool,
    attack_target_lost_presentation: bool,
    battle_alert_retreat: bool,
    battle_withdrawal_report: bool,
    battle_last_ship_lost: bool,
    umgul_backdrop: bool,
    coruscant_chase: bool,
    yavin_multi_ship: bool,
    battle_alert_open: bool,
    tactical_music_mdata_id: Option<u32>,
    tactical_weapon_audio_wave_first: u32,
    tactical_weapon_audio_wave_last: u32,
    tactical_weapon_audio_variant_count: u32,
    tactical_voice_variant_count: u32,
    subsystem_field_command_presentation: bool,
    live_subsystem_damage_presentation: bool,
    subsystem_repair_mobility_presentation: bool,
    maneuver_movement_presentation: bool,
    command_assignment_presentation: bool,
    command_execution_presentation: bool,
    command_progression_presentation: bool,
    attack_targeting_presentation: bool,
    attack_target_lifecycle_presentation: bool,
    death_star_presentation: bool,
    battle_results_presentation: bool,
    battle_options_presentation: bool,
    battle_options_withdrawal: bool,
    death_star_laser_journey: bool,
    trench_run_outcome: Option<&'a str>,
    trench_run_remaining_ms: Option<f32>,
    trench_run_group: Option<u8>,
    tactical_rng_state: u32,
    weapon_draws: u32,
    tactical_lod: &'a str,
    system: &'a str,
    system_picture_id: u8,
    planet_resource_id: u32,
    palette_resource_id: u32,
    attacker_ships: usize,
    defender_ships: usize,
    fighters: usize,
    source_layout: Option<FixtureSourceLayout>,
    participants: Vec<FixtureParticipant<'a>>,
    effects: Vec<FixtureEffect>,
    projectiles: Vec<FixtureProjectile>,
    fields: Vec<FixtureField>,
    subsystem_repairs: Vec<FixtureSubsystemRepair>,
    selected_ship: Option<FixtureSelectedShip<'a>>,
    death_star: Option<FixtureDeathStar>,
    error: Option<&'a str>,
}

#[derive(Serialize)]
struct FixtureDeathStar {
    faction: &'static str,
    is_attacker: bool,
    tactical_ordinal: u8,
    resource_id: u32,
    opposing_resource_id: u32,
    operational: bool,
    hull: f32,
    laser_charge: f32,
    destroyed: bool,
    action_committed: bool,
    source_position: [f32; 3],
}

#[derive(Serialize)]
struct FixtureSelectedShip<'a> {
    name: &'a str,
    faction: &'static str,
    class_dat_id: u32,
    hud_resource: Option<u32>,
    hull_current: i32,
    hull_max: i32,
    shield_current: i32,
    shield_max: i32,
    subsystem_percentages: [u8; 5],
    subsystem_resources: [u32; 5],
    subsystem_hit_limits: [u8; 5],
    subsystem_damage_hits: [u8; 5],
    base_engine_power: f32,
    engine_mode_bonus: f32,
    active_tractor_power: f32,
    effective_engine_power: f32,
    maneuverability: u32,
    maneuver_state_value: Option<f32>,
    movement_alignment: f32,
    source_position: [f32; 3],
    source_forward: [f32; 3],
    source_desired_forward: [f32; 3],
    source_velocity: [f32; 3],
    source_waypoint: Option<[f32; 3]>,
    order_code: u8,
    tactic_code: u8,
    damage_control: u8,
    contents: Vec<FixtureShipContent<'a>>,
}

#[derive(Serialize)]
struct FixtureShipContent<'a> {
    label: &'a str,
    resource_id: u32,
    icon_id: Option<u32>,
}

#[derive(Serialize)]
struct FixtureSubsystemRepair {
    ship: usize,
    kind: &'static str,
    hits_before: u8,
    hits_after: u8,
}

#[derive(Serialize)]
struct FixtureEffect {
    target: usize,
    resource_base: u32,
    resource_id: u32,
    frame: u8,
    frame_count: u8,
    source_size: [u16; 2],
}

#[derive(Serialize)]
struct FixtureProjectile {
    source: usize,
    target: usize,
    kind: &'static str,
    longitudinal_scale: f32,
    shape_variant: u8,
    color_selector: u8,
    elapsed: f32,
    duration: f32,
    progress: f32,
}

#[derive(Serialize)]
struct FixtureField {
    target: usize,
    /// Written only for a fighter group, so a ship's report keeps its
    /// former shape.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    target_is_fighter: bool,
    resource_id: u32,
    frame: u8,
    tractor_sources: u8,
    gravity_sources: u8,
    tractor_source_ids: Vec<usize>,
    gravity_source_ids: Vec<usize>,
}

#[derive(Serialize)]
struct FixtureSourceLayout {
    first_active_objects: u16,
    second_active_objects: u16,
    battle_extent: f32,
    outer_positive_z: f32,
    outer_negative_z: f32,
    inner_negative_z: f32,
    inner_positive_z: f32,
}

#[derive(Serialize)]
struct FixtureParticipant<'a> {
    kind: &'static str,
    name: &'a str,
    faction: &'static str,
    class_dat_id: u32,
    fleet_roster_index: usize,
    tactical_ordinal: Option<u8>,
    resource_base: Option<u32>,
    opposing_resource_base: Option<u32>,
    initial_close_resource: Option<u32>,
    initial_far_resource: Option<u32>,
    initial_indicator_resource: Option<u32>,
    task_force: Option<u8>,
    fighter_group: Option<u8>,
    active_close_resource: Option<u32>,
    active_far_resource: Option<u32>,
    active_indicator_resource: Option<u32>,
    active: bool,
    retreating: Option<bool>,
    source_position: [f32; 3],
    source_forward: Option<[f32; 3]>,
    source_desired_forward: Option<[f32; 3]>,
    source_waypoint: Option<[f32; 3]>,
    order_code: u8,
    tactic_code: u8,
    recovery_state_code: Option<u8>,
    recovery_target: Option<usize>,
    attack_target_kind: Option<&'static str>,
    attack_target_index: Option<usize>,
}

/// A0 TAC-04-C014's sides: Fleet 2, one Alliance Escort Carrier with one
/// X-wing and one Y-wing squadron (A0 TAC-02-C002 "X-wings: 1 12/12",
/// TAC-02-C003 Blue Group), and the Imperial fleet the galaxy placed at
/// Coruscant as the defender. port: the galaxy's seeded home fleet stands in
/// for the day-2 save's, which the manifest names only in part (Judicator,
/// TIE squadrons).
/// The TIE squadrons on Coruscant in A0's save (A0 manifest TAC-05-C009:
/// "the 11 Coruscant planet TIE squadrons").
const CORUSCANT_PLANET_TIE_SQUADRONS: u32 = 11;

fn coruscant_chase_fleets(
    world: &mut GameWorld,
    system: SystemKey,
) -> Result<(rebellion_core::ids::FleetKey, rebellion_core::ids::FleetKey), String> {
    let defender = world.systems[system]
        .fleets
        .iter()
        .copied()
        .find(|&fleet| {
            !world.fleets[fleet].is_alliance && !world.fleets[fleet].capital_ships.is_empty()
        })
        .ok_or("Coruscant has no Imperial fleet")?;
    // A0's Empire counted 12 objects: A0 TAC-04-C014's pyramids fit a battle
    // extent of 136 = 100 + 3 x 12 (FUN_005ab650, 0x5abb11..0x5abb4b), and
    // not 109 (ghidra/notes/tactical-chase-camera-distance.md). The count
    // leaves out units in state 1 (0x5ab94b -> FUN_005b69f0). A squadron
    // aboard a ship without a hyperdrive starts in its hangar in state 1
    // (0x40bf0b -> FUN_005964c0 -> FUN_005b69d0). A squadron on a planet
    // starts deployed and counts. The same save holds the Judicator with
    // its TIE squadrons aboard, and 11 TIE squadrons on Coruscant (A0
    // manifest TAC-02-C004, TAC-05-C009). So the 12 are the hull and the 11
    // planet squadrons.
    // The new game seeds 10 of them (CMUNCRTB, FUN_0051ae20 -> FUN_0051bf30);
    // port: the eleventh, which A0's day-2 save also holds, lands here too.
    // They stay on the planet: the battle entry sends them up
    // (squadrons::scramble, FUN_0040b0e0 0x40b7d5) as the campaign's does.
    let tie = world
        .fighter_classes
        .iter()
        .find(|(_, class)| !class.is_alliance && class.name == "TIE Fighter")
        .map(|(key, _)| key)
        .ok_or("no TIE Fighter class")?;
    let planet = &mut world.systems[system].fighters;
    let seeded = planet
        .iter()
        .filter(|entry| entry.class == tie)
        .map(|entry| entry.count)
        .sum::<u32>();
    rebellion_core::squadrons::add(
        planet,
        tie,
        CORUSCANT_PLANET_TIE_SQUADRONS.saturating_sub(seeded),
    );
    let (carrier, hull) = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| class.is_alliance && class.name == "Alliance Escort Carrier")
        .map(|(key, class)| (key, class.hull))
        .ok_or("no Alliance Escort Carrier class")?;
    let fighter = |name: &str| {
        world
            .fighter_classes
            .iter()
            .find(|(_, class)| class.is_alliance && class.name == name)
            .map(|(key, _)| key)
            .ok_or(format!("no {name} class"))
    };
    let fighters = [fighter("X-wing")?, fighter("Y-wing")?]
        .map(|class| FighterEntry {
            class,
            count: 1,
            carrier: None,
        })
        .to_vec();
    let attacker = world.fleets.insert(Fleet {
        location: system,
        capital_ships: vec![ShipInstance::new(
            carrier,
            i32::try_from(hull).unwrap_or(i32::MAX),
            true,
        )],
        fighters,
        characters: vec![],
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    Ok((attacker, defender))
}

/// A0 TAC-01-C007's sides, read from the Empire's save four days before the
/// battle was sent (VM `saves-orig/SAVEGAME.004`, day 131, read-only):
/// Imperial Fleet 1 holds Imperial Star Destroyer 1 (CAPSHPSD 133) with six
/// TIE Fighter squadrons (FIGHTSD 5) and three regiments, and Victory
/// Destroyer 2 (CAPSHPSD 132) with one squadron and one regiment. A0
/// TAC-03-C009/C015 show the two hulls in Task Force # 1 and one Imperial
/// fighter group lit. The Alliance's one hull in A0 TAC-01-C007 is Swift,
/// the first name of the Alliance's small-hull list (TEXTSTRA 0x5200,
/// `FUN_004d2b30`), a Corellian Corvette (CAPSHPSD 69) in the same save.
/// port: the regiments stay out; they take no part in the placement.
fn yavin_multi_ship_fleets(
    world: &mut GameWorld,
    system: SystemKey,
) -> Result<(rebellion_core::ids::FleetKey, rebellion_core::ids::FleetKey), String> {
    let class = |world: &GameWorld, dat: u32| {
        world
            .capital_ship_classes
            .iter()
            .find(|(_, class)| class.dat_id.index() == dat)
            .map(|(key, class)| (key, i32::try_from(class.hull).unwrap_or(i32::MAX)))
            .ok_or(format!("no CAPSHPSD {dat} class"))
    };
    let corvette = class(world, 69)?;
    let star_destroyer = class(world, 133)?;
    let victory = class(world, 132)?;
    let tie = world
        .fighter_classes
        .iter()
        .find(|(_, class)| class.dat_id.index() == 5)
        .map(|(key, _)| key)
        .ok_or("no FIGHTSD 5 class")?;
    let mut numbered = |(class, hull): (CapitalShipKey, i32), is_alliance: bool| {
        world.last_ship_id += 1;
        let mut ship = ShipInstance::new(class, hull, is_alliance);
        ship.id = world.last_ship_id;
        ship
    };
    let swift = numbered(corvette, true);
    let first = numbered(star_destroyer, false);
    let second = numbered(victory, false);
    // Each squadron rides its carrier, so it waits in its hangar
    // (`FUN_005964c0`: no hyperdrive).
    let fighters = [(6, first.id), (1, second.id)]
        .map(|(count, carrier)| FighterEntry {
            class: tie,
            count,
            carrier: Some(carrier),
        })
        .to_vec();
    let attacker = world.fleets.insert(Fleet {
        location: system,
        capital_ships: vec![swift],
        fighters: Vec::new(),
        characters: vec![],
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    let defender = world.fleets.insert(Fleet {
        location: system,
        capital_ships: vec![first, second],
        fighters,
        characters: vec![],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    Ok((attacker, defender))
}

fn fixture_attack_target(
    target: Option<TacticalAttackTarget>,
) -> (Option<&'static str>, Option<usize>) {
    match target {
        Some(TacticalAttackTarget::CapitalShip(index)) => (Some("capital-ship"), Some(index)),
        Some(TacticalAttackTarget::FighterGroup(index)) => (Some("fighter-group"), Some(index)),
        None => (None, None),
    }
}

pub(crate) fn emit_ready(request: TacticalFixtureRequest, tactical: &TacticalState) {
    let Some(session) = tactical.session.as_ref() else {
        emit_failed(request, "battle session missing after fixture entry");
        return;
    };
    let layout = session.source_layout;
    let mut participants = Vec::with_capacity(session.ships.len() + session.fighters.len());
    participants.extend(session.ships.iter().map(|ship| {
        let (attack_target_kind, attack_target_index) = fixture_attack_target(ship.attack_target);
        let tactical_ordinal = ship
            .tactical_resource
            .map(|resource| resource.tactical_ordinal)
            .or_else(|| {
                ship.death_star_resource
                    .map(|resource| resource.tactical_ordinal)
            });
        let resource_base = ship
            .tactical_resource
            .map(|resource| resource.mesh_resource_base)
            .or_else(|| {
                ship.death_star_resource
                    .map(|resource| resource.resource_base_without_flag)
            });
        FixtureParticipant {
            kind: "capital-ship",
            name: &ship.name,
            faction: if ship.identity.is_alliance {
                "alliance"
            } else {
                "empire"
            },
            class_dat_id: ship.identity.class_dat_id.index(),
            fleet_roster_index: ship.identity.fleet_roster_index,
            tactical_ordinal,
            resource_base,
            opposing_resource_base: ship
                .death_star_resource
                .map(|resource| resource.resource_base_with_flag),
            initial_close_resource: None,
            initial_far_resource: None,
            initial_indicator_resource: None,
            task_force: Some(ship.task_force),
            fighter_group: None,
            active_close_resource: None,
            active_far_resource: None,
            active_indicator_resource: None,
            active: ship.alive,
            retreating: Some(ship.retreating),
            source_position: [
                ship.source_position.x,
                ship.source_position.y,
                ship.source_position.z,
            ],
            source_forward: Some([
                ship.source_forward.x,
                ship.source_forward.y,
                ship.source_forward.z,
            ]),
            source_desired_forward: Some([
                ship.source_desired_forward.x,
                ship.source_desired_forward.y,
                ship.source_desired_forward.z,
            ]),
            source_waypoint: ship
                .source_waypoint
                .map(|waypoint| [waypoint.x, waypoint.y, waypoint.z]),
            order_code: ship.order.source_code(),
            tactic_code: ship.tactic.source_code(),
            recovery_state_code: None,
            recovery_target: None,
            attack_target_kind,
            attack_target_index,
        }
    }));
    participants.extend(session.fighters.iter().map(|fighter| {
        let (attack_target_kind, attack_target_index) =
            fixture_attack_target(fighter.attack_target);
        let player_side = fighter.is_attacker == session.player_is_attacker;
        FixtureParticipant {
            kind: "fighter-group",
            name: &fighter.name,
            faction: if fighter.identity.is_alliance {
                "alliance"
            } else {
                "empire"
            },
            class_dat_id: fighter.identity.class_dat_id.index(),
            fleet_roster_index: fighter.identity.fleet_roster_index,
            tactical_ordinal: fighter
                .tactical_resource
                .map(|resource| resource.tactical_ordinal),
            resource_base: fighter
                .tactical_resource
                .map(|resource| resource.first_side_resource_base),
            opposing_resource_base: fighter
                .tactical_resource
                .map(|resource| resource.opposing_side_resource_base),
            initial_close_resource: fighter
                .tactical_resource
                .map(|resource| resource.initial_close_resource(player_side)),
            initial_far_resource: fighter
                .tactical_resource
                .map(|resource| resource.initial_far_resource(player_side)),
            initial_indicator_resource: fighter
                .tactical_resource
                .map(|resource| resource.initial_indicator_resource(player_side)),
            task_force: None,
            fighter_group: Some(fighter.fighter_group),
            active_close_resource: fighter.tactical_resource.map(|resource| {
                resource.grouped_close_resource(player_side, fighter.fighter_group)
            }),
            active_far_resource: fighter
                .tactical_resource
                .map(|resource| resource.grouped_far_resource(player_side, fighter.fighter_group)),
            active_indicator_resource: fighter.tactical_resource.map(|resource| {
                resource.grouped_indicator_resource(player_side, fighter.fighter_group)
            }),
            active: fighter.alive,
            retreating: None,
            source_position: [
                fighter.source_position.x,
                fighter.source_position.y,
                fighter.source_position.z,
            ],
            source_forward: None,
            source_desired_forward: None,
            source_waypoint: None,
            order_code: fighter.order.source_code(),
            tactic_code: fighter.tactic.source_code(),
            recovery_state_code: Some(fighter.recovery_state.source_code()),
            recovery_target: fighter.recovery_target,
            attack_target_kind,
            attack_target_index,
        }
    }));
    let effects = session
        .impact_effects
        .iter()
        .map(|effect| {
            let (width, height) = effect.sequence.source_size();
            FixtureEffect {
                target: effect.target,
                resource_base: effect.sequence.resource_base(),
                resource_id: effect.resource_id(),
                frame: effect.shown_frame,
                frame_count: effect.sequence.frame_count(),
                source_size: [width, height],
            }
        })
        .collect();
    let projectiles = session
        .weapon_effects
        .iter()
        .map(|effect| FixtureProjectile {
            source: effect.source,
            target: effect.target,
            kind: match effect.kind {
                WeaponKind::LaserCannon => "laser",
                WeaponKind::Turbolaser => "turbolaser",
                WeaponKind::IonCannon => "ion",
                WeaponKind::FighterAttack => "torpedo",
            },
            longitudinal_scale: effect.longitudinal_scale,
            shape_variant: effect.shape_variant,
            color_selector: effect.color_selector,
            elapsed: effect.elapsed,
            duration: effect.duration,
            progress: effect.progress(),
        })
        .collect();
    let fields = session
        .field_effects
        .iter()
        .filter_map(|effect| {
            let (tractor_sources, gravity_sources) = effect.source_counts();
            let (tractor_source_ids, gravity_source_ids) = effect.source_ids();
            Some(FixtureField {
                target: match effect.target {
                    TacticalUnitRef::Ship(index) | TacticalUnitRef::Fighter(index) => index,
                },
                target_is_fighter: matches!(effect.target, TacticalUnitRef::Fighter(_)),
                resource_id: effect.resource_id()?,
                frame: effect.frame,
                tractor_sources,
                gravity_sources,
                tractor_source_ids: tractor_source_ids.to_vec(),
                gravity_source_ids: gravity_source_ids.to_vec(),
            })
        })
        .collect();
    let subsystem_repairs = session
        .subsystem_repairs
        .iter()
        .map(|repair| FixtureSubsystemRepair {
            ship: repair.ship,
            kind: repair.kind.label(),
            hits_before: repair.hits_before,
            hits_after: repair.hits_after,
        })
        .collect();
    let selected_ship = session.selected_ship.and_then(|index| {
        let ship = session.ships.get(index)?;
        let mobility = session.subsystem_mobility(index)?;
        Some(FixtureSelectedShip {
            name: &ship.name,
            faction: if ship.identity.is_alliance {
                "alliance"
            } else {
                "empire"
            },
            class_dat_id: ship.identity.class_dat_id.index(),
            hud_resource: ship
                .tactical_resource
                .map(|resource| resource.hud_resource()),
            hull_current: ship.hull_current,
            hull_max: ship.hull_max,
            shield_current: ship.shield,
            shield_max: ship.shield_max,
            subsystem_percentages: ship.subsystem_condition.percentages(),
            subsystem_resources: ship.subsystem_condition.resource_ids(),
            subsystem_hit_limits: ship.subsystem_capacity.hit_limits(),
            subsystem_damage_hits: ship.subsystem_damage.hits(),
            base_engine_power: mobility.base_engine_power,
            engine_mode_bonus: mobility.engine_mode_bonus,
            active_tractor_power: mobility.active_tractor_power,
            effective_engine_power: mobility.effective_engine_power,
            maneuverability: ship.maneuverability,
            maneuver_state_value: ship.maneuver_state_value,
            movement_alignment: session.movement_alignment(index).unwrap_or(0.0),
            source_position: [
                ship.source_position.x,
                ship.source_position.y,
                ship.source_position.z,
            ],
            source_forward: [
                ship.source_forward.x,
                ship.source_forward.y,
                ship.source_forward.z,
            ],
            source_desired_forward: [
                ship.source_desired_forward.x,
                ship.source_desired_forward.y,
                ship.source_desired_forward.z,
            ],
            source_velocity: [
                ship.source_velocity.x,
                ship.source_velocity.y,
                ship.source_velocity.z,
            ],
            source_waypoint: ship
                .source_waypoint
                .map(|waypoint| [waypoint.x, waypoint.y, waypoint.z]),
            order_code: ship.order.source_code(),
            tactic_code: ship.tactic.source_code(),
            damage_control: ship.damage_control,
            contents: ship
                .contents
                .iter()
                .map(|content| FixtureShipContent {
                    label: content.label.as_str(),
                    resource_id: content.resource_id,
                    icon_id: tactical_content_icon(content.resource_id),
                })
                .collect(),
        })
    });
    let death_star = session.death_star.map(|death_star| FixtureDeathStar {
        faction: if death_star.is_alliance {
            "alliance"
        } else {
            "empire"
        },
        is_attacker: death_star.is_attacker,
        tactical_ordinal: death_star.resource.tactical_ordinal,
        resource_id: death_star.resource_id(),
        opposing_resource_id: death_star.resource.resource_base_with_flag,
        operational: death_star.operational(),
        hull: death_star.hull,
        laser_charge: death_star.laser_charge,
        destroyed: death_star.destroyed,
        action_committed: death_star.action_committed,
        source_position: [
            death_star.source_position.x,
            death_star.source_position.y,
            death_star.source_position.z,
        ],
    });
    emit(&FixtureRecord {
        schema_version: 36,
        status: "battle-ready",
        family: "tactical",
        fixture_code: request.code,
        faction: if request.faction == CockpitFaction::Alliance {
            "alliance"
        } else {
            "empire"
        },
        proof_enabled: request.proof_enabled,
        production_participants: request.production_participants,
        suppress_capital_fallback: request.suppress_capital_fallback,
        focus_player_fighter: request.focus_player_fighter,
        group_presentation: request.group_presentation,
        effect_presentation: request.effect_presentation,
        projectile_field_presentation: request.projectile_field_presentation,
        selected_damage_presentation: request.selected_damage_presentation,
        selected_navigation_presentation: request.selected_navigation_presentation,
        navigation_camera_presentation: request.navigation_camera_presentation,
        empty_space_presentation: request.empty_space_presentation,
        detail_escort_presentation: request.detail_escort_presentation,
        battle_alert_presentation: request.battle_alert_presentation,
        tactical_audio_presentation: request.tactical_audio_presentation,
        hull_scale_presentation: request.hull_scale_presentation,
        attack_target_lost_presentation: request.attack_target_lost_presentation,
        battle_alert_retreat: request.battle_alert_retreat,
        battle_withdrawal_report: request.battle_withdrawal_report,
        battle_last_ship_lost: request.battle_last_ship_lost,
        umgul_backdrop: request.umgul_backdrop,
        coruscant_chase: request.coruscant_chase,
        yavin_multi_ship: request.yavin_multi_ship,
        battle_alert_open: tactical.battle_alert_open(),
        tactical_music_mdata_id: None,
        tactical_weapon_audio_wave_first: 13_000,
        tactical_weapon_audio_wave_last: 13_053,
        tactical_weapon_audio_variant_count: 36,
        tactical_voice_variant_count: 285,
        subsystem_field_command_presentation: request.subsystem_field_command_presentation,
        live_subsystem_damage_presentation: request.live_subsystem_damage_presentation,
        subsystem_repair_mobility_presentation: request.subsystem_repair_mobility_presentation,
        maneuver_movement_presentation: request.maneuver_movement_presentation,
        command_assignment_presentation: request.command_assignment_presentation,
        command_execution_presentation: request.command_execution_presentation,
        command_progression_presentation: request.command_progression_presentation,
        attack_targeting_presentation: request.attack_targeting_presentation,
        attack_target_lifecycle_presentation: request.attack_target_lifecycle_presentation,
        death_star_presentation: request.death_star_presentation,
        battle_results_presentation: request.battle_results_presentation,
        battle_options_presentation: request.battle_options_presentation,
        battle_options_withdrawal: request.battle_options_withdrawal,
        death_star_laser_journey: request.death_star_laser_journey,
        trench_run_outcome: session.trench_run_outcome.map(|outcome| match outcome {
            TacticalTrenchRunOutcome::Success => "success",
            TacticalTrenchRunOutcome::Failure => "failure",
        }),
        trench_run_remaining_ms: session.trench_run_remaining_ms,
        trench_run_group: session.trench_run_group,
        tactical_rng_state: session.tactical_rng.state(),
        weapon_draws: session.weapon_draws,
        tactical_lod: request.lod_fixture.label(),
        system: &session.system_name,
        system_picture_id: session.system_picture_id,
        planet_resource_id: 5500 + u32::from(session.system_picture_id),
        palette_resource_id: 5530 + u32::from(session.system_picture_id),
        attacker_ships: session.ships.iter().filter(|ship| ship.is_attacker).count(),
        defender_ships: session
            .ships
            .iter()
            .filter(|ship| !ship.is_attacker)
            .count(),
        fighters: session.fighters.len(),
        source_layout: Some(FixtureSourceLayout {
            first_active_objects: layout.first_active_objects,
            second_active_objects: layout.second_active_objects,
            battle_extent: layout.battle_extent,
            outer_positive_z: layout.outer_positive_z,
            outer_negative_z: layout.outer_negative_z,
            inner_negative_z: layout.inner_negative_z,
            inner_positive_z: layout.inner_positive_z,
        }),
        participants,
        effects,
        projectiles,
        fields,
        subsystem_repairs,
        selected_ship,
        death_star,
        error: None,
    });
}

pub(crate) fn emit_failed(request: TacticalFixtureRequest, error: &str) {
    emit(&FixtureRecord {
        schema_version: 36,
        status: "failed",
        family: "tactical",
        fixture_code: request.code,
        faction: if request.faction == CockpitFaction::Alliance {
            "alliance"
        } else {
            "empire"
        },
        proof_enabled: request.proof_enabled,
        production_participants: request.production_participants,
        suppress_capital_fallback: request.suppress_capital_fallback,
        focus_player_fighter: request.focus_player_fighter,
        group_presentation: request.group_presentation,
        effect_presentation: request.effect_presentation,
        projectile_field_presentation: request.projectile_field_presentation,
        selected_damage_presentation: request.selected_damage_presentation,
        selected_navigation_presentation: request.selected_navigation_presentation,
        navigation_camera_presentation: request.navigation_camera_presentation,
        empty_space_presentation: request.empty_space_presentation,
        detail_escort_presentation: request.detail_escort_presentation,
        battle_alert_presentation: request.battle_alert_presentation,
        tactical_audio_presentation: request.tactical_audio_presentation,
        hull_scale_presentation: request.hull_scale_presentation,
        attack_target_lost_presentation: request.attack_target_lost_presentation,
        battle_alert_retreat: request.battle_alert_retreat,
        battle_withdrawal_report: request.battle_withdrawal_report,
        battle_last_ship_lost: request.battle_last_ship_lost,
        umgul_backdrop: request.umgul_backdrop,
        coruscant_chase: request.coruscant_chase,
        yavin_multi_ship: request.yavin_multi_ship,
        battle_alert_open: false,
        tactical_music_mdata_id: None,
        tactical_weapon_audio_wave_first: 13_000,
        tactical_weapon_audio_wave_last: 13_053,
        tactical_weapon_audio_variant_count: 36,
        tactical_voice_variant_count: 285,
        subsystem_field_command_presentation: request.subsystem_field_command_presentation,
        live_subsystem_damage_presentation: request.live_subsystem_damage_presentation,
        subsystem_repair_mobility_presentation: request.subsystem_repair_mobility_presentation,
        maneuver_movement_presentation: request.maneuver_movement_presentation,
        command_assignment_presentation: request.command_assignment_presentation,
        command_execution_presentation: request.command_execution_presentation,
        command_progression_presentation: request.command_progression_presentation,
        attack_targeting_presentation: request.attack_targeting_presentation,
        attack_target_lifecycle_presentation: request.attack_target_lifecycle_presentation,
        death_star_presentation: request.death_star_presentation,
        battle_results_presentation: request.battle_results_presentation,
        battle_options_presentation: request.battle_options_presentation,
        battle_options_withdrawal: request.battle_options_withdrawal,
        death_star_laser_journey: request.death_star_laser_journey,
        trench_run_outcome: None,
        trench_run_remaining_ms: None,
        trench_run_group: None,
        tactical_rng_state: 0,
        weapon_draws: 0,
        tactical_lod: request.lod_fixture.label(),
        system: "",
        system_picture_id: 0,
        planet_resource_id: 0,
        palette_resource_id: 0,
        attacker_ships: 0,
        defender_ships: 0,
        fighters: 0,
        source_layout: None,
        participants: Vec::new(),
        effects: Vec::new(),
        projectiles: Vec::new(),
        fields: Vec::new(),
        subsystem_repairs: Vec::new(),
        selected_ship: None,
        death_star: None,
        error: Some(error),
    });
}

fn emit(record: &FixtureRecord<'_>) {
    let bytes = serde_json::to_vec(record).expect("serialize tactical fixture record");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tactical_codes_are_versioned_and_do_not_accept_gid_codes() {
        assert_eq!(decode(0x10101).unwrap().faction, CockpitFaction::Alliance);
        assert_eq!(decode(0x10201).unwrap().faction, CockpitFaction::Empire);
        assert!(decode(0x10101).unwrap().proof_enabled);
        assert!(!decode(0x10102).unwrap().proof_enabled);
        assert!(decode(0x10108).unwrap().production_participants);
        assert!(decode(0x10109).unwrap().suppress_capital_fallback);
        assert!(decode(0x1010a).unwrap().production_participants);
        assert!(decode(0x1010b).unwrap().group_presentation);
        assert!(decode(0x1010c).unwrap().effect_presentation);
        assert!(decode(0x1010d).unwrap().projectile_field_presentation);
        assert!(decode(0x1010e).unwrap().selected_damage_presentation);
        assert!(
            decode(0x1010f)
                .unwrap()
                .subsystem_field_command_presentation
        );
        assert!(decode(0x10110).unwrap().live_subsystem_damage_presentation);
        assert!(
            decode(0x10111)
                .unwrap()
                .subsystem_repair_mobility_presentation
        );
        assert!(decode(0x10112).unwrap().maneuver_movement_presentation);
        assert!(decode(0x10113).unwrap().command_assignment_presentation);
        assert!(decode(0x10114).unwrap().command_execution_presentation);
        assert!(decode(0x10115).unwrap().command_progression_presentation);
        assert!(decode(0x10116).unwrap().attack_targeting_presentation);
        assert!(
            decode(0x10117)
                .unwrap()
                .attack_target_lifecycle_presentation
        );
        assert!(decode(0x10118).unwrap().death_star_presentation);
        assert!(decode(0x10118).unwrap().focus_player_fighter);
        assert!(decode(0x10119).unwrap().battle_results_presentation);
        assert!(decode(0x1011a).unwrap().battle_options_presentation);
        assert!(!decode(0x1011a).unwrap().battle_options_withdrawal);
        assert!(decode(0x1011b).unwrap().battle_options_presentation);
        assert!(decode(0x1011b).unwrap().battle_options_withdrawal);
        assert!(decode(0x1011c).unwrap().death_star_presentation);
        assert!(decode(0x1011c).unwrap().death_star_laser_journey);
        assert_eq!(
            decode(0x1011d).unwrap().trench_run_outcome,
            Some(TacticalTrenchRunOutcome::Success)
        );
        assert_eq!(
            decode(0x1011e).unwrap().trench_run_outcome,
            Some(TacticalTrenchRunOutcome::Failure)
        );
        assert!(decode(0x1011f).unwrap().selected_navigation_presentation);
        assert!(decode(0x10120).unwrap().navigation_camera_presentation);
        assert!(decode(0x10121).unwrap().empty_space_presentation);
        assert!(decode(0x10122).unwrap().detail_escort_presentation);
        assert!(decode(0x10123).unwrap().battle_alert_presentation);
        assert!(decode(0x10124).unwrap().tactical_audio_presentation);
        assert!(decode(0x10125).unwrap().hull_scale_presentation);
        assert!(decode(0x10126).unwrap().attack_target_lost_presentation);
        assert!(decode(0x10126).unwrap().attack_target_lifecycle_presentation);
        assert!(!decode(0x10117).unwrap().attack_target_lost_presentation);
        assert!(decode(0x10127).unwrap().battle_alert_retreat);
        assert!(!decode(0x10127).unwrap().battle_alert_presentation);
        assert!(!decode(0x10123).unwrap().battle_alert_retreat);
        assert!(decode(0x10128).unwrap().battle_withdrawal_report);
        assert!(!decode(0x10127).unwrap().battle_withdrawal_report);
        assert_eq!(
            decode(0x10103).unwrap().lod_fixture,
            TacticalLodFixture::Close
        );
        assert_eq!(
            decode(0x10104).unwrap().lod_fixture,
            TacticalLodFixture::Medium
        );
        assert_eq!(
            decode(0x10105).unwrap().lod_fixture,
            TacticalLodFixture::Far
        );
        assert_eq!(
            decode(0x10106).unwrap().lod_fixture,
            TacticalLodFixture::Journey
        );
        assert_eq!(
            decode(0x10107).unwrap().lod_fixture,
            TacticalLodFixture::CameraJourney
        );
        assert!(decode(0x10100).is_none());
        assert!(decode(0x10129).unwrap().battle_last_ship_lost);
        assert!(decode(0x10129).unwrap().hull_scale_presentation);
        assert!(!decode(0x10128).unwrap().battle_last_ship_lost);
        assert!(decode(0x1012a).unwrap().umgul_backdrop);
        assert!(!decode(0x10129).unwrap().umgul_backdrop);
        assert!(decode(0x1012c).unwrap().battle_last_ship_lost);
        assert!(decode(0x1012c).unwrap().summary_story_movie);
        assert!(!decode(0x10129).unwrap().summary_story_movie);
        assert!(decode(0x1012d).unwrap().yavin_multi_ship);
        assert!(!decode(0x1012c).unwrap().yavin_multi_ship);
        assert!(decode(0x1012e).is_none());
        assert!(decode(0x10301).is_none());
        assert!(decode(0x00101).is_none());
    }

    #[test]
    fn code_45_enters_only_the_yavin_multi_ship_battle_on_either_side() {
        for (code, faction) in [
            (0x1012d, CockpitFaction::Alliance),
            (0x1022d, CockpitFaction::Empire),
        ] {
            let request = decode(code).unwrap();
            assert_eq!(request.faction, faction);
            assert!(request.yavin_multi_ship);
            assert!(request.production_participants);
            assert!(!request.coruscant_chase && !request.hull_scale_presentation);
            assert!(!request.empty_space_presentation);
        }
    }

    // A0 TAC-01-C007 / TAC-01-C009 (manifest
    // new-game-empire-small-2026-10-24-yavin; sides from the VM's
    // SAVEGAME.004): the TIE squadrons wait in their carriers' hangars
    // (FUN_005964c0, FUN_005b84a0), so FUN_005ab650 counts two Imperial
    // hulls against one: extent 106. The Star Destroyer leads Task Force 1
    // at x 0 and FUN_005cbbc0 moves the Victory onto its Stand Off place,
    // 12 on the +x side (FUN_005ec400, FUN_005ea030), where A0's frame
    // shows it.
    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn the_a0_yavin_battle_keeps_its_ties_aboard_and_the_victory_twelve_beside_the_star_destroyer(
    ) {
        use rebellion_core::tactical::{BattleSession, TacticalWorldPosition};
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
        let mut world = rebellion_data::load_game_data(&data).expect("load original game data");
        let system = world
            .systems
            .iter()
            .find(|(_, system)| system.name == "Yavin")
            .map(|(key, _)| key)
            .unwrap();
        world.systems[system].fleets.clear();
        let (attacker, defender) = yavin_multi_ship_fleets(&mut world, system).unwrap();
        world.systems[system].fleets.extend([attacker, defender]);
        let session = BattleSession::new(&world, system, attacker, defender, false, 0, 1);
        assert_eq!(session.source_layout.battle_extent, 106.0);
        let names: Vec<_> = session
            .ships
            .iter()
            .map(|ship| ship.identity.class_dat_id.index())
            .collect();
        assert_eq!(names, [69, 133, 132]);
        let at = |x: f32, z: f32| TacticalWorldPosition { x, y: 0.0, z };
        assert_eq!(session.ships[0].source_position, at(0.0, -53.0));
        assert_eq!(session.ships[1].source_position, at(0.0, 53.0));
        assert_eq!(session.ships[2].source_position, at(12.0, 53.0));
        assert_eq!((session.ships[1].task_force, session.ships[2].task_force), (0, 0));
        assert_eq!(session.fighters.len(), 7);
        assert!(session.fighters.iter().all(|fighter| fighter.in_hangar && !fighter.alive));
        assert_eq!((session.ships[1].hangar.len(), session.ships[2].hangar.len()), (6, 1));
        // FUN_005ae460: seven squadrons of one class make one group.
        let group = session.fighters[0].fighter_group;
        assert!(session.fighters.iter().all(|fighter| fighter.fighter_group == group));
    }

    #[test]
    fn code_43_enters_only_the_coruscant_chase_battle_on_either_side() {
        for (code, faction) in [
            (0x1012b, CockpitFaction::Alliance),
            (0x1022b, CockpitFaction::Empire),
        ] {
            let request = decode(code).unwrap();
            assert_eq!(request.faction, faction);
            assert!(request.coruscant_chase);
            assert!(request.production_participants);
            assert!(!request.umgul_backdrop && !request.hull_scale_presentation);
        }
        assert!(!decode(0x1012a).unwrap().coruscant_chase);
    }

    // A0 TAC-04-C014 (manifest new-game-alliance-small-2026-10-24-coruscant2):
    // Fleet 2 is one Alliance Escort Carrier with an X-wing and a Y-wing
    // squadron, and it attacks the Imperial fleet at Coruscant.
    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn the_coruscant_chase_sends_an_escort_carrier_with_x_and_y_wings_against_coruscants_fleet() {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
        let mut world = rebellion_data::load_game_data(&data).expect("load original game data");
        let system = world
            .systems
            .iter()
            .find(|(_, system)| system.name == "Coruscant")
            .map(|(key, _)| key)
            .unwrap();
        let home = world.systems[system].fleets.clone();
        let (attacker, defender) = coruscant_chase_fleets(&mut world, system).unwrap();
        assert!(home.contains(&defender));
        assert!(!world.fleets[defender].is_alliance);
        let fleet = &world.fleets[attacker];
        assert!(fleet.is_alliance);
        assert_eq!(fleet.location, system);
        let ships: Vec<_> = fleet
            .capital_ships
            .iter()
            .map(|ship| world.capital_ship_classes[ship.class].name.as_str())
            .collect();
        assert_eq!(ships, ["Alliance Escort Carrier"]);
        let fighters: Vec<_> = fleet
            .fighters
            .iter()
            .map(|entry| {
                (
                    world.fighter_classes[entry.class].name.as_str(),
                    entry.count,
                )
            })
            .collect();
        assert_eq!(fighters, [("X-wing", 1), ("Y-wing", 1)]);
    }

    // A0 TAC-04-C014's chase view fits a battle extent of 136, 100 + 3 x 12
    // (FUN_005ab650, 0x5abb11..0x5abb4b; ghidra/notes/
    // tactical-chase-camera-distance.md): the Star Destroyer and Coruscant's
    // 11 TIE squadrons, whose home-fleet squadrons stay in its hangar
    // (state 1, 0x40bf0b -> FUN_005b69d0). The 11 stand on the planet and
    // the battle takes them from there (FUN_0040b0e0, 0x40b7d5), through the
    // same scramble and side fleets as the fixture's entry.
    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn the_coruscant_chase_battle_counts_twelve_imperial_objects_as_a0_s_extent_shows() {
        use rebellion_core::abstract_battle::side_fleets;
        use rebellion_core::tactical::BattleSession;
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
        // The browser build, where the gate runs, seeds its galaxy with 0
        // (seeds::default_seed on wasm32); a native load seeds from the clock.
        let options = rebellion_core::world::SeedOptions {
            rng_seed: Some(0),
            ..rebellion_core::world::SeedOptions::default()
        };
        let mut world = rebellion_data::load_game_data_with_options(&data, &options)
            .expect("load original game data");
        let system = world
            .systems
            .iter()
            .find(|(_, system)| system.name == "Coruscant")
            .map(|(key, _)| key)
            .unwrap();
        let (attacker, defender) = coruscant_chase_fleets(&mut world, system).unwrap();
        world.systems[system].fleets.push(attacker);
        let on_planet: Vec<_> = world.systems[system]
            .fighters
            .iter()
            .map(|entry| {
                (
                    world.fighter_classes[entry.class].name.as_str(),
                    entry.count,
                    entry.carrier,
                )
            })
            .collect();
        assert_eq!(on_planet, [("TIE Fighter", 11, None)]);
        assert!(world.fleets[defender]
            .fighters
            .iter()
            .all(|entry| entry.carrier.is_some()));
        let sorties = rebellion_core::squadrons::scramble(&mut world, system);
        assert_eq!(sorties.len(), 1);
        let session = BattleSession::new_for_fleets(
            &world,
            system,
            &side_fleets(&world, system, attacker),
            &side_fleets(&world, system, defender),
            false,
            0,
            1,
        )
        .session;
        let layout = session.source_layout;
        assert_eq!(
            (layout.first_active_objects, layout.second_active_objects),
            (3, 12)
        );
        assert_eq!(layout.battle_extent, 136.0);
        assert_eq!(layout.outer_positive_z, 68.0);
        let deployed: Vec<_> = session
            .fighters
            .iter()
            .filter(|fighter| !fighter.identity.is_alliance && fighter.alive)
            .map(|fighter| world.fighter_classes[fighter.class_key].name.as_str())
            .collect();
        assert_eq!(deployed, ["TIE Fighter"; 11]);
    }
}
