//! Query-gated browser runner for the native/WASM replay-equivalence fixture.

use std::collections::HashMap;
use std::path::Path;

use macroquad::file::load_file;
use rebellion_data::replay::{compute_simulation_data_manifest, SimulationDataManifest};
use rebellion_data::replay_fixture::{
    run_seed42_gate, seed42_seed_options, ReplayGateReport, SEED42_ARTIFACT_BYTES,
};
use rebellion_data::replay_sweep::{run_sweep_gate, SweepReport, SWEEP_DAYS, SWEEP_SEEDS};

const REPLAY_MODE_ABSENT: u32 = 0;
const REPLAY_MODE_SEED42_V1: u32 = 1;
const REPLAY_MODE_SWEEP_V1: u32 = 3;

extern "C" {
    fn open_rebellion_replay_mode() -> u32;
    fn open_rebellion_replay_emit(ptr: *const u8, len: usize);
}

pub fn requested() -> bool {
    unsafe { open_rebellion_replay_mode() != REPLAY_MODE_ABSENT }
}

fn emit(report: &impl serde::Serialize) {
    let json = serde_json::to_vec(report).expect("serialize browser replay-gate report");
    unsafe { open_rebellion_replay_emit(json.as_ptr(), json.len()) };
}

pub async fn run(data_path: &Path) {
    let mode = unsafe { open_rebellion_replay_mode() };
    let (passed, detail) = match mode {
        REPLAY_MODE_SEED42_V1 => {
            let report = run_seed42_from_runtime_pack(data_path).await;
            emit(&report);
            let detail = report
                .error
                .clone()
                .unwrap_or_else(|| "all nine checkpoints matched".to_string());
            (report.passed(), detail)
        }
        REPLAY_MODE_SWEEP_V1 => {
            let report = run_sweep_from_runtime_pack(data_path).await;
            emit(&report);
            let detail = report
                .error
                .clone()
                .unwrap_or_else(|| "every seed's burst and spread runs agreed".to_string());
            (report.passed(), detail)
        }
        _ => {
            let report = ReplayGateReport::failure(
                "wasm32",
                "query",
                format!("unsupported replay-check query mode {mode}"),
            );
            emit(&report);
            (false, report.error.unwrap_or_default())
        }
    };

    loop {
        macroquad::prelude::clear_background(macroquad::prelude::Color::new(0.02, 0.02, 0.06, 1.0));
        macroquad::prelude::draw_text(
            if passed {
                "Replay equivalence: PASS"
            } else {
                "Replay equivalence: FAIL"
            },
            32.0,
            58.0,
            30.0,
            if passed {
                macroquad::prelude::GREEN
            } else {
                macroquad::prelude::RED
            },
        );
        macroquad::prelude::draw_text(&detail, 32.0, 92.0, 18.0, macroquad::prelude::LIGHTGRAY);
        macroquad::prelude::next_frame().await;
    }
}

/// Fetch the runtime pack, fingerprint its `.DAT` inputs and install its
/// files and strings for the loader. Returns the failing phase and error.
async fn install_runtime_pack() -> Result<SimulationDataManifest, (&'static str, String)> {
    let bytes = load_file("data/runtime.orpk").await.map_err(|error| {
        (
            "runtime_pack_fetch",
            format!("data/runtime.orpk failed to load: {error:?}"),
        )
    })?;
    let mut pack = crate::runtime_pack::parse_owned_runtime_pack(bytes)
        .map_err(|error| ("runtime_pack_decode", error.to_string()))?;
    let data = compute_simulation_data_manifest(
        pack.game_files
            .iter()
            .filter(|(name, _)| name.to_ascii_uppercase().ends_with(".DAT")),
    )
    .map_err(|error| ("data_manifest", format!("{error:#}")))?;
    let strings: HashMap<u16, String> = match pack.game_files.remove("textstra.json") {
        Some(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| ("string_table", format!("textstra.json is malformed: {error}")))?,
        None => {
            return Err((
                "string_table",
                "runtime pack is missing textstra.json".to_string(),
            ))
        }
    };
    rebellion_data::set_string_table(strings);
    rebellion_data::set_file_cache(pack.game_files);
    Ok(data)
}

async fn run_sweep_from_runtime_pack(data_path: &Path) -> SweepReport {
    let data = match install_runtime_pack().await {
        Ok(data) => data,
        Err((phase, error)) => return SweepReport::failure("wasm32", phase, error),
    };
    run_sweep_gate("wasm32", &data, SWEEP_SEEDS, SWEEP_DAYS, |options| {
        rebellion_data::load_game_data_with_options(data_path, options)
    })
}

async fn run_seed42_from_runtime_pack(data_path: &Path) -> ReplayGateReport {
    let data = match install_runtime_pack().await {
        Ok(data) => data,
        Err((phase, error)) => return ReplayGateReport::failure("wasm32", phase, error),
    };

    let options = seed42_seed_options();
    let world = match rebellion_data::load_game_data_with_options(data_path, &options) {
        Ok(world) => world,
        Err(error) => {
            return ReplayGateReport::failure("wasm32", "game_data", format!("{error:#}"))
        }
    };
    run_seed42_gate("wasm32", SEED42_ARTIFACT_BYTES, &data, world)
}
