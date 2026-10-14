# Seed-42 replay golden restored (F-011)

Date: 2026-10-14. Runner: Claude Code, local Linux checkout.

## Cause

The golden last regenerated in `e7808e5f` (2026-10-07) stopped reproducing:

1. `7b34b8c1` made the loader honour the chosen galaxy size. The fixture used
   the default (`GalaxySize::Standard`), so it loaded 100 systems and failed
   its own 200-system guard before replaying anything.
2. Later simulation and save changes moved every fingerprint: save v33
   `ProductionState` (`0729220e`), refined material and yard cycles
   (`ed1fdbeb`, `6eb24155`, `5ac49a9b`), seed table thresholds (`e2a900be`),
   story messages (`d034c1ec`, `1c197a9f`), and the 2026-10-14 overnight
   changes (`fb89f98a`, `d6adacfe`, `78c39acd`, `5d163de0`).

## Fix

`replay_fixture::seed42_seed_options()` is the one place the fixture's seed
options live. It selects `GalaxySize::Huge`, the full 200-system galaxy the
fixture has always covered. The ignored native test, `replay-gate`, and the
browser runner (`web_replay.rs`) all use it. The campaign config inside the
initial state now also records the large galaxy, where it used to say
Standard while holding 200 systems.

Golden: `v1:def9666124cd1433` -> `v1:a079b9347c1c574e` becomes
`v1:24c73c2d80dea3f6` -> `v1:01685e44ab44b004`. Data identity
(`5facb1c7ba0e81ad`, 51 inputs, 50,597 bytes) and the configuration
fingerprint are unchanged. The artifact stays 13,479 bytes.

## Results

| Check | Result |
|---|---|
| Recorder run twice in separate processes | byte-identical artifacts |
| `cargo test -p rebellion-data --test replay_manifest -- --ignored` | 2 passed (failed before the fix: "requires 200 systems, loaded 100") |
| `cargo test -p rebellion-data replay --lib` | 14 passed |
| `replay-gate data/base` (`native-report.json`) | passed, tick 25, nine checkpoints |
| Browser `?replay-check=seed42-v1` (`browser-report.json`) | passed; status, initial fingerprint, all nine checkpoints, final tick, final fingerprint, data fingerprint, and artifact text equal the native report |
| Browser requests (`browser-network.json`) | 4 of 4 HTTP 200 |
| Browser console (`browser-console.log`) | no errors or exceptions |
| Screenshot (`browser-replay.png`, inspected) | "Replay equivalence: PASS, all nine checkpoints matched" |

The browser run used `cdp-replay.mjs` (headless Chromium 153.0.8010.52 with
SwiftShader, driven over CDP from Node 22.22.0) because `agent-browser` is not
installed on this machine. It covers the main equivalence case only; the
invalid-query, missing-pack, and normal-startup cases in
`scripts/check-replay-equivalence.py` were not rerun.

`web/open-rebellion.wasm` is an unoptimized build (`wasm-opt` is not
installed). Hashes are in `SHA256SUMS`.
