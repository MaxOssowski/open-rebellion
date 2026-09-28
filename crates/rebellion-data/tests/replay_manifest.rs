//! Native fixture proof for the platform-neutral replay data manifest.

use std::path::PathBuf;

use rebellion_data::replay::{
    compute_simulation_data_manifest_from_dir, execute_replay, record_replay, ReplayEnvironment,
    ReplayManifest,
};
use rebellion_data::replay_fixture::{
    seed42_commands, seed42_initial_state, validate_seed42_artifact, SEED42_ARTIFACT_BYTES,
    SEED42_ENGINE_VERSION, SEED42_FINAL_FINGERPRINT, SEED42_FINAL_TICK, SEED42_INITIAL_FINGERPRINT,
    SEED42_SEED,
};
use rebellion_data::save::{compute_state_fingerprint, load_slot, save_slot};

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory")
        .parent()
        .expect("repository root")
        .join("data/base")
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn original_simulation_data_has_canonical_identity() {
    let manifest =
        compute_simulation_data_manifest_from_dir(&data_dir()).expect("fingerprint original DATs");

    assert_eq!(manifest.inputs.len(), 51);
    assert_eq!(manifest.total_bytes, 50_597);
    assert_eq!(manifest.aggregate_fingerprint.value, "5facb1c7ba0e81ad");
    assert_eq!(manifest.inputs.first().unwrap().name, "ABDCMSTB.DAT");
    assert_eq!(manifest.inputs.last().unwrap().name, "UPRIS2TB.DAT");
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn original_campaign_replay_matches_after_save_reload() {
    let seed = SEED42_SEED;
    let data =
        compute_simulation_data_manifest_from_dir(&data_dir()).expect("fingerprint original DATs");
    let environment = ReplayEnvironment {
        engine_version: SEED42_ENGINE_VERSION,
        seed,
        data: &data,
    };
    let world = rebellion_data::load_game_data_with_options(
        &data_dir(),
        &rebellion_core::world::SeedOptions {
            rng_seed: Some(seed),
            ..rebellion_core::world::SeedOptions::default()
        },
    )
    .expect("load original campaign data");
    let initial = seed42_initial_state(world).expect("build seed-42 initial state");
    let initial_fingerprint = compute_state_fingerprint(&initial).unwrap();
    let recording =
        record_replay(environment, initial.clone(), seed42_commands()).expect("record replay");
    validate_seed42_artifact(&recording.manifest).expect("validate reviewed replay profile");
    let artifact = ReplayManifest::from_json(SEED42_ARTIFACT_BYTES)
        .expect("decode exact committed replay artifact bytes");
    validate_seed42_artifact(&artifact).expect("validate committed replay artifact");
    assert_eq!(artifact, recording.manifest);

    let saves = tempfile::tempdir().expect("temporary save directory");
    save_slot(saves.path(), 0, "Replay Start", &initial, &[]).expect("save initial state");
    let (_, restored) = load_slot(saves.path(), 0).expect("reload initial state");
    let executed = execute_replay(environment, &artifact, restored)
        .expect("execute exact committed replay artifact");
    let executed_fingerprint = compute_state_fingerprint(&executed.final_state).unwrap();
    let recorded_fingerprint = compute_state_fingerprint(&recording.execution.final_state).unwrap();

    assert_eq!(
        executed.observed_checkpoints,
        recording.manifest.checkpoints
    );
    assert_eq!(executed_fingerprint, recorded_fingerprint);
    assert_eq!(executed.final_state.clock.tick, SEED42_FINAL_TICK);
    assert_eq!(initial_fingerprint.to_string(), SEED42_INITIAL_FINGERPRINT);
    let observed_fingerprints: Vec<_> = recording
        .manifest
        .checkpoints
        .iter()
        .map(|checkpoint| {
            (
                checkpoint.command_count,
                checkpoint.tick,
                checkpoint.state_fingerprint.clone(),
            )
        })
        .collect();
    assert_eq!(
        observed_fingerprints,
        vec![
            (1, 0, "v1:60e105ab2fcd487a".into()),
            (2, 0, "v1:0cd0657a4ccebb6c".into()),
            (3, 5, "v1:b8c8b0a02f7be63c".into()),
            (4, 10, "v1:24acfab2e20dc9e9".into()),
            (5, 15, "v1:adacfb5d2d3893b2".into()),
            (6, 20, "v1:cf91858faee9ed51".into()),
            (7, 25, "v1:74ace7e853cbe2ab".into()),
            (8, 25, "v1:6e43534d24f38473".into()),
            (9, 25, "v1:6e43534d24f38473".into()),
        ]
    );
    assert_eq!(executed_fingerprint.to_string(), SEED42_FINAL_FINGERPRINT);
}
