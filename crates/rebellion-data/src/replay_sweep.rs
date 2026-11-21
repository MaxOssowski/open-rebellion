//! Multi-seed native/WASM determinism sweep (roadmap M2).
//!
//! The seed-42 fixture ([`crate::replay_fixture`]) proves one reviewed
//! 25-day artifact. The sweep runs longer dual-AI campaigns on several seeds
//! with each day's AI passes both in one burst (the headless runner, replays)
//! and spread over the day's sub-ticks (the interactive app,
//! `rebellion_core::ai_schedule`), and stages one automatic space battle and
//! one headless tactical battle. Native and WASM runners publish the same
//! [`SweepReport`]; a cross-runtime gate compares them field by field.
//!
//! The sweep keeps no committed goldens: every simulation change moves its
//! fingerprints, and its claim is equality between runtimes and schedules,
//! which each run checks itself.

use anyhow::{bail, Context};
use rebellion_core::game_events::{GameEventRecord, EVT_COMBAT_GROUND, EVT_COMBAT_SPACE};
use rebellion_core::ids::{FleetKey, SystemKey};
use rebellion_core::tactical::{BattlePhase, BattleSession};
use rebellion_core::tick::GameSpeed;
use rebellion_core::world::{GameWorld, SeedOptions};
use serde::{Deserialize, Serialize};

use crate::replay::{AiPassSchedule, CampaignDayRunner, ReplayCommand, SimulationDataManifest};
use crate::replay_fixture::{campaign_initial_state, campaign_seed_options};

pub const SWEEP_FIXTURE_ID: &str = "sweep-v1";
/// Campaign seeds the sweep runs.
pub const SWEEP_SEEDS: &[u64] = &[1, 2, 3];
/// Days each sweep campaign runs.
pub const SWEEP_DAYS: u64 = 200;
/// A state fingerprint is sampled every this many days.
pub const SWEEP_SAMPLE_EVERY: u64 = 25;
/// The seed whose galaxy stages the battle check.
pub const SWEEP_BATTLE_SEED: u64 = 42;
/// The headless tactical battle's own generator seed.
pub const SWEEP_TACTICAL_RNG_SEED: u32 = 0x4242_0001;
/// Tactical steps run at most (about 25 minutes at four steps a second).
pub const SWEEP_TACTICAL_STEP_LIMIT: u32 = 6_000;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// One state fingerprint taken at the end of a day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SweepSample {
    pub tick: u64,
    pub state_fingerprint: String,
}

/// One campaign run with one AI pass schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SweepRun {
    pub seed: u64,
    pub schedule: AiPassSchedule,
    pub days: u64,
    pub initial_fingerprint: String,
    pub samples: Vec<SweepSample>,
    pub final_fingerprint: String,
    pub ai_orders: u64,
    pub events: u64,
    /// FNV-1a over every event's tick, system, type and details.
    pub event_digest: String,
    pub space_battles: u64,
    pub ground_battles: u64,
}

/// One automatic battle and one headless tactical battle between the same
/// staged fleets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SweepBattle {
    pub seed: u64,
    pub system_dat_id: u32,
    pub alliance_ships: usize,
    pub empire_ships: usize,
    /// The automatic battle's `combat_space` event details.
    pub auto_results: Vec<serde_json::Value>,
    pub auto_fingerprint: String,
    pub tactical_steps: u32,
    pub tactical_ended: bool,
    pub tactical_ships_alive: [usize; 2],
    /// FNV-1a over the tactical session's ships and fighters after each
    /// hundredth step and at the end (their `Debug` text, floats included).
    pub tactical_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SweepReport {
    pub schema_version: u16,
    pub fixture_id: String,
    pub platform: String,
    pub status: String,
    pub data_input_count: usize,
    pub data_bytes: u64,
    pub data_fingerprint: String,
    pub runs: Vec<SweepRun>,
    pub battle: Option<SweepBattle>,
    pub failure_phase: Option<String>,
    pub error: Option<String>,
}

impl SweepReport {
    #[must_use]
    pub fn failure(platform: &str, phase: &str, error: impl Into<String>) -> Self {
        Self {
            schema_version: 1,
            fixture_id: SWEEP_FIXTURE_ID.to_string(),
            platform: platform.to_string(),
            status: "failed".to_string(),
            data_input_count: 0,
            data_bytes: 0,
            data_fingerprint: String::new(),
            runs: Vec::new(),
            battle: None,
            failure_phase: Some(phase.to_string()),
            error: Some(error.into()),
        }
    }

    #[must_use]
    pub fn passed(&self) -> bool {
        self.status == "passed"
    }
}

struct Fnv(u64);

impl Fnv {
    const fn new() -> Self {
        Self(FNV_OFFSET_BASIS)
    }

    fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(FNV_PRIME);
        }
    }

    fn framed(&mut self, bytes: &[u8]) {
        self.update(&(bytes.len() as u64).to_le_bytes());
        self.update(bytes);
    }

    fn hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

fn digest_event(digest: &mut Fnv, event: &GameEventRecord) {
    // The wall clock is left out: it is the host's, not the simulation's.
    digest.update(&event.tick.to_le_bytes());
    digest.framed(event.system.as_bytes());
    digest.framed(event.event_type.as_bytes());
    digest.framed(event.details.to_string().as_bytes());
}

/// A dual-AI campaign runner at Medium speed, as the spread-vs-burst test
/// runs it.
fn dual_ai_runner(world: GameWorld, seed: u64) -> anyhow::Result<CampaignDayRunner> {
    let initial = campaign_initial_state(world, seed)?;
    let mut runner = CampaignDayRunner::new(initial);
    runner.apply(&ReplayCommand::SetSpeed {
        speed: GameSpeed::Medium,
    })?;
    runner.apply(&ReplayCommand::ToggleDualAi)?;
    Ok(runner)
}

/// Run `days` days of `seed`'s campaign with the AI passes on `schedule`.
///
/// # Errors
/// Returns an error if the campaign cannot start or a state cannot encode.
pub fn run_sweep_campaign(
    world: GameWorld,
    seed: u64,
    days: u64,
    schedule: AiPassSchedule,
) -> anyhow::Result<SweepRun> {
    let mut runner = dual_ai_runner(world, seed)?;
    let initial_fingerprint = runner.fingerprint()?.to_string();
    let mut digest = Fnv::new();
    let mut run = SweepRun {
        seed,
        schedule,
        days,
        initial_fingerprint,
        samples: Vec::new(),
        final_fingerprint: String::new(),
        ai_orders: 0,
        events: 0,
        event_digest: String::new(),
        space_battles: 0,
        ground_battles: 0,
    };
    for day in 1..=days {
        let outcome = runner.run_day(schedule);
        let ai_day = runner.ai_day();
        if ai_day.done != ai_day.passes || ai_day.owes_close() {
            bail!(
                "seed {seed} day {day}: {} of {} AI passes ran",
                ai_day.done,
                ai_day.passes
            );
        }
        run.ai_orders += u64::from(outcome.ai_orders);
        run.events += outcome.events.len() as u64;
        for event in &outcome.events {
            digest_event(&mut digest, event);
            if event.event_type == EVT_COMBAT_SPACE {
                run.space_battles += 1;
            } else if event.event_type == EVT_COMBAT_GROUND {
                run.ground_battles += 1;
            }
        }
        if day % SWEEP_SAMPLE_EVERY == 0 || day == days {
            run.samples.push(SweepSample {
                tick: outcome.tick,
                state_fingerprint: runner.fingerprint()?.to_string(),
            });
        }
    }
    run.final_fingerprint = runner.fingerprint()?.to_string();
    run.event_digest = digest.hex();
    Ok(run)
}

/// The fleet of `alliance`'s side with the most capital ships (the first in
/// key order on a tie).
fn largest_fleet(world: &GameWorld, alliance: bool) -> Option<FleetKey> {
    let mut best: Option<(FleetKey, usize)> = None;
    for (key, fleet) in &world.fleets {
        if fleet.is_alliance != alliance || fleet.has_death_star {
            continue;
        }
        let ships = fleet.capital_ships.len();
        if ships > 0 && best.is_none_or(|(_, most)| ships > most) {
            best = Some((key, ships));
        }
    }
    best.map(|(key, _)| key)
}

/// Move `fleet` into orbit at `system` (a staging step, not a game order).
fn place_fleet(world: &mut GameWorld, fleet: FleetKey, system: SystemKey) {
    let from = world.fleets[fleet].location;
    world.systems[from].fleets.retain(|&key| key != fleet);
    world.fleets[fleet].location = system;
    world.systems[system].fleets.push(fleet);
}

fn digest_tactical(digest: &mut Fnv, session: &BattleSession) {
    digest.update(&session.combat_tick.to_le_bytes());
    digest.framed(format!("{:?}", session.ships).as_bytes());
    digest.framed(format!("{:?}", session.fighters).as_bytes());
}

/// Stage `seed`'s largest Empire fleet at the largest Alliance fleet's
/// system, fight it once headless on the tactical engine with no input,
/// then let the next day resolve the same battle automatically.
///
/// # Errors
/// Returns an error if the galaxy has no fleets to stage or no battle
/// follows.
pub fn run_sweep_battle(world: GameWorld, seed: u64) -> anyhow::Result<SweepBattle> {
    let mut runner = CampaignDayRunner::new(campaign_initial_state(world, seed)?);
    let alliance = largest_fleet(runner.world(), true).context("no Alliance fleet with ships")?;
    let empire = largest_fleet(runner.world(), false).context("no Empire fleet with ships")?;
    let system = runner.world().fleets[alliance].location;
    place_fleet(runner.world_mut(), empire, system);
    let world = runner.world();
    let alliance_fleets: Vec<FleetKey> = world.systems[system]
        .fleets
        .iter()
        .copied()
        .filter(|&key| world.fleets[key].is_alliance)
        .collect();
    let count_ships = |fleets: &[FleetKey]| -> usize {
        fleets
            .iter()
            .map(|&key| world.fleets[key].capital_ships.len())
            .sum()
    };

    // The tactical engine on the same forces: the Alliance attacks as the
    // player, and each step is one fixed combat tick.
    let mut session = BattleSession::new_for_fleets(
        world,
        system,
        &alliance_fleets,
        &[empire],
        true,
        1,
        SWEEP_TACTICAL_RNG_SEED,
    )
    .session;
    // Observe Battle (command 0x7b, FUN_0059fd40): the player's commander
    // fights the player's side too, so the battle runs to its end.
    session.set_player_observing(true);
    // A battle opens paused under its Battle Alert; closing the alert
    // starts it.
    session.paused = false;
    let mut tactical = Fnv::new();
    let mut steps = 0;
    let mut ended = false;
    while steps < SWEEP_TACTICAL_STEP_LIMIT {
        ended = session.step();
        steps += 1;
        if steps % 100 == 0 {
            digest_tactical(&mut tactical, &session);
        }
        if ended || session.phase != BattlePhase::Combat {
            break;
        }
    }
    digest_tactical(&mut tactical, &session);
    let alive = |attacker: bool| {
        session
            .ships
            .iter()
            .filter(|ship| ship.is_attacker == attacker && ship.alive)
            .count()
    };
    let tactical_ships_alive = [alive(true), alive(false)];
    if !ended {
        bail!("the staged tactical battle did not end in {SWEEP_TACTICAL_STEP_LIMIT} steps");
    }

    let mut battle = SweepBattle {
        seed,
        system_dat_id: world.systems[system].dat_id.raw(),
        alliance_ships: count_ships(&alliance_fleets),
        empire_ships: count_ships(&[empire]),
        auto_results: Vec::new(),
        auto_fingerprint: String::new(),
        tactical_steps: steps,
        tactical_ended: ended,
        tactical_ships_alive,
        tactical_digest: tactical.hex(),
    };

    // The automatic battle: the next day's combat step finds both sides.
    let day = runner.run_day(AiPassSchedule::Burst);
    let system_name = runner.world().systems[system].name.clone();
    battle.auto_results = day
        .events
        .iter()
        .filter(|event| {
            event.event_type == EVT_COMBAT_SPACE
                && event
                    .details
                    .get("system")
                    .and_then(serde_json::Value::as_str)
                    == Some(system_name.as_str())
        })
        .map(|event| event.details.clone())
        .collect();
    if battle.auto_results.is_empty() {
        bail!("the staged fleets fought no automatic battle at {system_name}");
    }
    battle.auto_fingerprint = runner.fingerprint()?.to_string();
    Ok(battle)
}

/// Whether two runs of one seed on different AI schedules reached the same
/// states, orders and events.
fn schedules_agree(burst: &SweepRun, spread: &SweepRun) -> bool {
    burst.initial_fingerprint == spread.initial_fingerprint
        && burst.samples == spread.samples
        && burst.final_fingerprint == spread.final_fingerprint
        && burst.ai_orders == spread.ai_orders
        && burst.event_digest == spread.event_digest
}

/// Run the whole sweep: every seed in both schedules, then the battle.
/// `load` builds a galaxy from seed options on the host's own file access.
///
/// A run fails if the spread and burst schedules disagree on any sample,
/// the final state, the events or the orders.
#[must_use]
pub fn run_sweep_gate<F>(
    platform: &str,
    data: &SimulationDataManifest,
    seeds: &[u64],
    days: u64,
    mut load: F,
) -> SweepReport
where
    F: FnMut(&SeedOptions) -> anyhow::Result<GameWorld>,
{
    let mut report = SweepReport {
        schema_version: 1,
        fixture_id: SWEEP_FIXTURE_ID.to_string(),
        platform: platform.to_string(),
        status: "failed".to_string(),
        data_input_count: data.inputs.len(),
        data_bytes: data.total_bytes,
        data_fingerprint: data.aggregate_fingerprint.value.clone(),
        runs: Vec::new(),
        battle: None,
        failure_phase: None,
        error: None,
    };
    let outcome = (|| -> anyhow::Result<()> {
        for &seed in seeds {
            report.failure_phase = Some(format!("seed_{seed}"));
            let options = campaign_seed_options(seed);
            let burst = run_sweep_campaign(load(&options)?, seed, days, AiPassSchedule::Burst)?;
            let spread = run_sweep_campaign(load(&options)?, seed, days, AiPassSchedule::Spread)?;
            let agree = schedules_agree(&burst, &spread);
            report.runs.push(burst);
            report.runs.push(spread);
            if !agree {
                bail!("seed {seed}: the spread AI schedule diverged from the burst");
            }
        }
        report.failure_phase = Some("battle".to_string());
        let options = campaign_seed_options(SWEEP_BATTLE_SEED);
        report.battle = Some(run_sweep_battle(load(&options)?, SWEEP_BATTLE_SEED)?);
        Ok(())
    })();
    match outcome {
        Ok(()) => {
            report.status = "passed".to_string();
            report.failure_phase = None;
        }
        Err(error) => report.error = Some(format!("{error:#}")),
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_event_digest_ignores_the_wall_clock_but_not_the_details() {
        let record = |wall_ms, hull| {
            GameEventRecord::new(
                7,
                wall_ms,
                "combat",
                EVT_COMBAT_SPACE,
                serde_json::json!({ "hull": hull }),
            )
        };
        let digest = |event: &GameEventRecord| {
            let mut digest = Fnv::new();
            digest_event(&mut digest, event);
            digest.hex()
        };
        assert_eq!(digest(&record(1, 5)), digest(&record(900, 5)));
        assert_ne!(digest(&record(1, 5)), digest(&record(1, 6)));
    }

    #[test]
    fn a_framed_digest_tells_apart_moved_boundaries() {
        let mut one = Fnv::new();
        one.framed(b"ab");
        one.framed(b"c");
        let mut two = Fnv::new();
        two.framed(b"a");
        two.framed(b"bc");
        assert_ne!(one.hex(), two.hex());
    }

    fn original_data() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base")
    }

    fn sweep(seeds: &[u64], days: u64) -> SweepReport {
        let dir = original_data();
        let data = crate::replay::compute_simulation_data_manifest_from_dir(&dir).unwrap();
        run_sweep_gate("native", &data, seeds, days, |options| {
            crate::load_game_data_with_options(&dir, options)
        })
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn the_sweep_runs_the_same_twice_in_one_process() {
        // port: the burst and spread schedules must agree
        // (rebellion_core::ai_schedule); two runs in one process also catch
        // state that follows a HashMap's per-instance iteration order.
        let first = sweep(&[1], 60);
        assert!(first.passed(), "{first:?}");
        assert_eq!(first, sweep(&[1], 60));
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_sweep_run_counts_what_its_days_report() {
        // port: the run's totals and samples against the same days counted
        // here from the runner directly.
        let dir = original_data();
        let load = || crate::load_game_data_with_options(&dir, &campaign_seed_options(1)).unwrap();
        // Seed 1's first ground battle falls on day 635 (day 457 before
        // module 9 was ported, day 323 before module 0xa, day 1 to 210
        // under its stand-in).
        let run = run_sweep_campaign(load(), 1, 640, AiPassSchedule::Burst).unwrap();

        let mut runner = dual_ai_runner(load(), 1).unwrap();
        assert_eq!(
            run.initial_fingerprint,
            runner.fingerprint().unwrap().to_string()
        );
        let (mut orders, mut events, mut space, mut ground) = (0, 0, 0, 0);
        for _ in 0..640 {
            let day = runner.run_day(AiPassSchedule::Burst);
            orders += u64::from(day.ai_orders);
            events += day.events.len() as u64;
            space += day
                .events
                .iter()
                .filter(|e| e.event_type == EVT_COMBAT_SPACE)
                .count() as u64;
            ground += day
                .events
                .iter()
                .filter(|e| e.event_type == EVT_COMBAT_GROUND)
                .count() as u64;
        }
        assert!(orders > 0 && events > 0 && ground > 0);
        assert_eq!(
            (
                run.ai_orders,
                run.events,
                run.space_battles,
                run.ground_battles
            ),
            (orders, events, space, ground)
        );
        let ticks: Vec<u64> = run.samples.iter().map(|sample| sample.tick).collect();
        assert_eq!(
            ticks,
            [
                25, 50, 75, 100, 125, 150, 175, 200, 225, 250, 275, 300, 325, 350, 375, 400, 425,
                450, 475, 500, 525, 550, 575, 600, 625, 640
            ]
        );
        let last = runner.fingerprint().unwrap().to_string();
        assert_eq!(run.samples[25].state_fingerprint, last);
        assert_eq!(run.final_fingerprint, last);
        assert_ne!(
            run.samples[0].state_fingerprint,
            run.samples[1].state_fingerprint
        );
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn the_staged_fleets_are_each_side_s_first_largest() {
        // port: the battle's staging picks, per side, the first fleet in key
        // order with the most capital ships.
        let world = crate::load_game_data_with_options(
            &original_data(),
            &campaign_seed_options(SWEEP_BATTLE_SEED),
        )
        .unwrap();
        for alliance in [true, false] {
            let side: Vec<(FleetKey, usize)> = world
                .fleets
                .iter()
                .filter(|(_, fleet)| fleet.is_alliance == alliance && !fleet.has_death_star)
                .map(|(key, fleet)| (key, fleet.capital_ships.len()))
                .collect();
            let most = side.iter().map(|&(_, ships)| ships).max().unwrap();
            let first = side.iter().find(|&&(_, ships)| ships == most).unwrap().0;
            assert_eq!(largest_fleet(&world, alliance), Some(first));
        }

        // Staging moves only the staged fleet out of its system.
        let mut world = world;
        let empire = largest_fleet(&world, false).unwrap();
        let target = world.fleets[largest_fleet(&world, true).unwrap()].location;
        let from = world.fleets[empire].location;
        let mut stays = world.systems[from].fleets.clone();
        stays.retain(|&key| key != empire);
        place_fleet(&mut world, empire, target);
        assert_eq!(world.systems[from].fleets, stays);
        assert_eq!(world.fleets[empire].location, target);
        assert!(world.systems[target].fleets.contains(&empire));
    }

    #[test]
    fn two_runs_agree_only_when_every_compared_field_matches() {
        let run = SweepRun {
            seed: 1,
            schedule: AiPassSchedule::Burst,
            days: 1,
            initial_fingerprint: "v1:a".into(),
            samples: vec![SweepSample {
                tick: 1,
                state_fingerprint: "v1:b".into(),
            }],
            final_fingerprint: "v1:b".into(),
            ai_orders: 3,
            events: 4,
            event_digest: "c".into(),
            space_battles: 0,
            ground_battles: 0,
        };
        let spread = SweepRun {
            schedule: AiPassSchedule::Spread,
            ..run.clone()
        };
        assert!(schedules_agree(&run, &spread));
        let changes: [fn(&mut SweepRun); 5] = [
            |run| run.initial_fingerprint.push('x'),
            |run| run.samples[0].tick = 2,
            |run| run.final_fingerprint.push('x'),
            |run| run.ai_orders += 1,
            |run| run.event_digest.push('x'),
        ];
        for change in changes {
            let mut other = spread.clone();
            change(&mut other);
            assert!(!schedules_agree(&run, &other));
        }
    }

    #[test]
    fn a_sweep_whose_galaxy_fails_to_load_fails() {
        let data = crate::replay::compute_simulation_data_manifest([("A.DAT", b"a")]).unwrap();
        let report = run_sweep_gate("native", &data, &[1], 1, |_| bail!("no galaxy"));
        assert!(!report.passed());
        assert_eq!(report.failure_phase.as_deref(), Some("seed_1"));
        assert_eq!(report.error.as_deref(), Some("no galaxy"));
    }

    #[test]
    fn the_digest_is_fnv_1a_64() {
        // port: FNV-1a 64 of "a" (the published test vector).
        let mut digest = Fnv::new();
        digest.update(b"a");
        assert_eq!(digest.hex(), "af63dc4c8601ec8c");
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_staged_battle_ends_on_both_engines() {
        // port: FUN_005ae9c0 resolves the battle automatically; the tactical
        // engine with Observe Battle (FUN_0059fd40) runs it to its end.
        let report = sweep(&[], 0);
        assert!(report.passed(), "{report:?}");
        let battle = report.battle.unwrap();
        assert_eq!(battle.auto_results.len(), 1);
        assert!(battle.auto_results[0].get("winner").is_some(), "{battle:?}");
        assert!(battle.tactical_ended);
        assert!(battle.tactical_steps > 100 && battle.tactical_steps < SWEEP_TACTICAL_STEP_LIMIT);
        assert_ne!(battle.tactical_digest, Fnv::new().hex());
        let alive: usize = battle.tactical_ships_alive.iter().sum();
        assert!(
            alive < battle.alliance_ships + battle.empire_ships,
            "{battle:?}"
        );
    }
}
