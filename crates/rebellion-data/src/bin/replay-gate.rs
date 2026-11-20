use std::path::PathBuf;

use rebellion_data::replay::compute_simulation_data_manifest_from_dir;
use rebellion_data::replay_fixture::{run_seed42_gate, seed42_seed_options, SEED42_ARTIFACT_BYTES};
use rebellion_data::replay_sweep::{run_sweep_gate, SWEEP_DAYS, SWEEP_SEEDS};

/// `replay-gate [DATA_DIR] [--sweep]`: the seed-42 fixture, or with
/// `--sweep` the multi-seed sweep (`rebellion_data::replay_sweep`).
fn main() {
    let mut sweep = false;
    let mut data_dir = PathBuf::from("data/base");
    for argument in std::env::args_os().skip(1) {
        if argument == "--sweep" {
            sweep = true;
        } else {
            data_dir = PathBuf::from(argument);
        }
    }
    let data = match compute_simulation_data_manifest_from_dir(&data_dir) {
        Ok(data) => data,
        Err(error) => {
            eprintln!("failed to fingerprint {}: {error:#}", data_dir.display());
            std::process::exit(1);
        }
    };
    if sweep {
        let report = run_sweep_gate("native", &data, SWEEP_SEEDS, SWEEP_DAYS, |options| {
            rebellion_data::load_game_data_with_options(&data_dir, options)
        });
        println!(
            "{}",
            serde_json::to_string(&report).expect("serialize native sweep report")
        );
        if !report.passed() {
            std::process::exit(1);
        }
        return;
    }
    let options = seed42_seed_options();
    let world = match rebellion_data::load_game_data_with_options(&data_dir, &options) {
        Ok(world) => world,
        Err(error) => {
            eprintln!("failed to load {}: {error:#}", data_dir.display());
            std::process::exit(1);
        }
    };
    let report = run_seed42_gate("native", SEED42_ARTIFACT_BYTES, &data, world);
    println!(
        "{}",
        serde_json::to_string(&report).expect("serialize native replay-gate report")
    );
    if !report.passed() {
        std::process::exit(1);
    }
}
