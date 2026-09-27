//! Runs the built binary against the real workspace.

use std::process::Command;

#[test]
fn the_binary_prints_the_workspace_summary_and_passes_its_own_baseline() {
    let bin = env!("CARGO_BIN_EXE_provenance-scan");

    let report = Command::new(bin).arg("report").output().unwrap();
    assert!(report.status.success());
    let stdout = String::from_utf8(report.stdout).unwrap();
    assert!(stdout.starts_with("file (items with hits)"), "{stdout}");
    assert!(
        stdout.contains("crates/rebellion-core/src/missions.rs"),
        "{stdout}"
    );

    let check = Command::new(bin).arg("check").output().unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    assert!(String::from_utf8(check.stdout).unwrap().starts_with("ok: "));
}
