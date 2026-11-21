---
title: "Deterministic Replay Contract"
description: "Versioned command streams, simulation-data identity, checkpoints, and remaining cross-runtime gates"
category: agent-docs
created: 2026-09-09
updated: 2026-11-20
tags: [determinism, replay, wasm, multiplayer, testing]
---

# Deterministic Replay Contract

`rebellion-data/src/replay.rs` owns the portable replay artifact. It is the
boundary that native, browser, test, and eventual authoritative multiplayer
runners must share. The contract records the inputs needed to explain a run;
it does not yet make the app and playtest simulation loops equivalent.

## Current status

| Capability | Status | Evidence |
|---|---|---|
| Canonical save-state fingerprint | F-011A complete | Save fingerprint v1 |
| Exact post-load continuation state | F-011B1 complete | Save v10+ continuation envelope |
| Versioned replay JSON envelope | F-011B2 complete | Format v1 with strict decoding |
| Canonical `.DAT` identity | F-011B2 complete | Per-file and aggregate fingerprints |
| Gap-free command ordering | F-011B2 complete | `{tick, sequence, actor}` validation |
| Replay executor and recorder | F-011B3 complete | Per-command state checkpoints |
| Fresh-process native replay | F-011B3 complete | Stable 25-tick original-data golden |
| App/playtest tick convergence | Partial | One implementation of the day (`begin_day`, `start_ai_day`, `run_ai_passes`, `close_simulation_day`; F-012); same day rolls; the app's tactical battles are the one exception (`the_app_s_day_with_its_presentation_matches_the_headless_runner`) |
| Native/WASM checkpoint equality | F-011B4 complete | Exact seed-42 artifact, nine checkpoints |
| Native/WASM multi-seed equality | Complete 2026-11-20 | Sweep: seeds 1-3, 200 days, burst and spread AI schedules |
| App AI schedule (spread) = playtest (burst) | Complete 2026-11-20 | Native test and the sweep in both runtimes |
| Automatic battle native/WASM equality | Complete 2026-11-20 | Sweep's staged seed-42 battle |
| Tactical battle native/WASM equality | Complete 2026-11-20 | Same battle headless with Observe Battle; `portable_math` |
| Automatic = tactical outcome | Not comparable | Different engines (see below) |

## Format v2

Format v2 (2026-11-21) is v1 without the tuning configuration's
fingerprint, which went with `GameConfig` (save v135). `ReplayManifest`
contains:

- a format and engine version;
- the deterministic seed, RNG identity, and fixed roll budget;
- every simulation `.DAT` input in canonical filename order;
- the initial state fingerprint;
- ordered `ReplayCommandRecord` values; and
- expected `ReplayCheckpoint` values.

Each command has the simulation tick immediately before application, a
zero-based global sequence, an actor, and a typed command. Format v2 supports
the current headless control commands. Existing command semantics may not
change without incrementing `REPLAY_FORMAT_VERSION`.

Each checkpoint identifies a tick, the exact number of commands already
applied, and the expected versioned state fingerprint. This disambiguates
multiple commands at one tick and supports prefix-by-prefix failure reports.

## Recording and execution

`record_replay()` derives command ticks and gap-free sequence numbers from the
runtime, applies each typed command, and records a canonical `SaveState`
fingerprint after every command. `execute_replay()` independently validates the
engine version, seed, data manifest, active configuration, initial state,
command position, and each checkpoint. Playback stops at the first mismatch
with the checkpoint index, tick, command prefix, and expected/observed hashes.
`execute_replay_observed()` also reports the actual failing checkpoint to a
diagnostic caller without weakening fail-fast behavior.

Format v1 reserves exactly 1,024 Xoshiro256++ rolls per simulated tick and
limits one advance command to 1,000,000 ticks. The runtime supports speed
changes, dual-AI toggling, visibility controls, forced victory checks, and
explicit tick advances through the shared `run_simulation_tick()` path.

F-011B3 also removes process-random iteration from state-affecting
manufacturing completions, simultaneous fleet arrivals, blockade transitions,
and the AI's equal-count reinforcement choice. Regression tests require stable
key order at each boundary.

## Simulation-data identity

`compute_simulation_data_manifest()` accepts in-memory file bytes and therefore
runs identically in native and WASM builds. The native convenience function
reads every `.DAT` in a directory. Names are ASCII, flat, uppercased, sorted,
and unique. Non-DAT paths, traversal, case-colliding duplicates, malformed
hashes, inconsistent byte totals, and inconsistent aggregate hashes fail
closed.

Fingerprint v1 uses domain-separated FNV-1a 64 over length-framed names and
bytes. It is a deterministic equivalence and corruption signal, not a
cryptographic authenticator. Hash values serialize as fixed-width lowercase
hex strings so JavaScript cannot lose integer precision.

The local original-data fixture contains 51 inputs and 50,597 bytes. Its
aggregate fingerprint is `5facb1c7ba0e81ad`.

## Cross-runtime fixture

`crates/rebellion-data/tests/fixtures/replay_seed42_v1.json` is the single
reviewed artifact used by both runtimes. It contains nine commands and nine
checkpoints through tick 25. Native `replay-gate` and the query-gated WASM
runner both embed and decode those exact 13,479 bytes, rebuild the seed-42
campaign independently, and echo the original text into their reports.

The browser gate runs only at `?replay-check=seed42-v1`. It strictly loads
`runtime.orpk`, publishes `window.__openRebellionReplay`, and never falls into
normal gameplay. Invalid query values and a missing pack fail before any
partial replay state is reported. Without the query, ordinary startup uses
the same four requests, then fetches the intro movies (`data/video/000.webm`,
`001.webm`); the gate presses Escape over each, as the interface-parity gates
do, and requires the authentic bitmap menu.

## Multi-seed sweep

`rebellion-data/src/replay_sweep.rs` (fixture id `sweep-v1`) has no
committed goldens: every simulation change moves its fingerprints, and its
claim is equality. It runs, from the same `campaign_initial_state` the seed-42
fixture uses:

- seeds 1, 2 and 3 for 200 dual-AI days at Medium speed, once with each day's
  AI passes in one burst (`run_simulation_tick`, the playtest and replays)
  and once spread over the day's sub-ticks with the day's end after the last
  (`begin_simulation_day`, `run_ai_passes` per pass, `close_simulation_day`,
  as the app runs them). Each run samples the state fingerprint every 25
  days and digests every event (tick, system, type, details; not the wall
  clock). A seed fails unless both schedules agree on all of it;
- one staged battle: seed 42's largest Empire fleet is moved to the largest
  Alliance fleet's system. The tactical engine fights it headless with
  Observe Battle (`set_player_observing`, `FUN_0059fd40`) and the Battle
  Alert's pause lifted, one fixed `step()` at a time, digesting the ships
  and fighters every 100 steps and at the end; then the next day resolves
  the same battle automatically and the sweep keeps its `combat_space`
  details and the state fingerprint after it.

`replay-gate data/base --sweep` prints the native report;
`?replay-check=sweep-v1` publishes the browser's in
`window.__openRebellionReplay` (about 9 s in Chromium 157);
`check-replay-equivalence.py --sweep` compares them field by field.

The 200-day campaigns fight no space battle on seeds 1-3 (seed 1 has one
ground battle), so the staged battle is the sweep's only space combat.

### Platform floating point

The first sweep (2026-11-20) matched every campaign sample and the automatic
battle but not the tactical one: at step 122 a fighter's `x` was `0.4214537`
natively and `0.42145374` in WASM. `f32::hypot`, `sin_cos` and `acos` call the
C library natively (glibc here, the system libm on macOS) and Rust's libm port
in WebAssembly, and those round some last bits differently.
`rebellion_core::portable_math` replaces them in `tactical.rs` with versions
built from basic IEEE operations and square roots (FreeBSD/musl kernels for
sine, cosine and arc cosine, an `f64` sum for `hypot`), each within one ulp of
glibc. With it the whole sweep matches. The automatic battle
(`abstract_battle.rs`) and the strategic layer call no transcendental
function outside tests; keep it that way or route new calls through
`portable_math`.

### Automatic versus tactical

The automatic battle (`abstract_battle::resolve_space_battle`, FUN_005ae9c0)
and the tactical engine (`BattleSession::step`) are different engines and do
not produce the same state, even when they agree on the winner. In the
staged battle both now give the Empire the win and lose the Alliance's
corvette: the automatic battle has the Star Destroyer and its six TIE
squadrons come out untouched, while the tactical one ends with its own
damage and losses. Until the hangar squadrons launched at the abstract
battle's first step (FUN_005d14d0 / FUN_005af4b0, fix 4ae64b6c), the
automatic resolution had the lone Star Destroyer worn down and destroyed by
the corvette and Yavin's squadrons. The automatic battle also counts the planet's
squadrons (`squadrons::scramble`), which the tactical session built from the
fleets does not. What is comparable, and the sweep checks, is each engine's
result across runtimes for the same input. A played tactical battle also
depends on the player's commands and on wall-clock frames: the app steps
`4 × combat_speed` times a second from `get_frame_time()`
(`advance_tactical_frame`), so the step count reached by any moment differs
between machines. Its result is applied to the campaign outside the day loop
(`tactical_flow.rs`) and has no replay command yet; replaying a played battle
needs its commands recorded by step, which format v2 cannot express.

## Validation

```bash
cargo test -p rebellion-data replay --lib
cargo test -p rebellion-data --test replay_manifest -- --ignored
cargo check -p rebellion-app --target wasm32-unknown-unknown
python3 scripts/check-replay-equivalence.py --skip-build --json
python3 scripts/check-replay-equivalence.py --skip-build --sweep --json
cargo test -p rebellion-data --lib -- --ignored replay
```

Both scripts accept `--site DIR` to serve a staged copy (`index.html`,
`gl.js`, the WASM, `data/runtime.orpk`, `data/video`) instead of `web/`; build
the WASM with `cargo build --release --target wasm32-unknown-unknown -p
rebellion-app` into that copy rather than through `build-wasm.sh` when `web/`
belongs to another checkout.

The ignored fixture test requires the locally supplied original `.DAT` files.
It records a nine-command, 25-tick campaign on the full 200-system galaxy
(`seed42_seed_options()` selects `GalaxySize::Huge`; every seed-42 runner
must load through it), reloads its initial state through the current save
format, and checks every command-prefix
fingerprint against a cross-process golden, which currently runs
`v1:f9f829709bdbb3ed` -> `v1:c981cac8c020fff8`. Any save layout or
simulation change moves every fingerprint; regenerate the golden only for a
named cause and name it in the commit. The v13 goldens did not reproduce
even at their own commit `e8d4945`; that cause remains open. The unit tests use
synthetic data and run in normal repository test passes.

## Next implementation boundary

F-011B4 proves replay-executor equivalence for one 25-tick campaign, and the
sweep extends it to three seeds over 200 days, both AI schedules and one
battle on each combat engine. The app and the runner now share one day
(F-012), checked natively by
`the_app_s_day_with_its_presentation_matches_the_headless_runner`; the sweep
runs that day's AI passes spread in WASM but not the app's own presentation
host. Still open: player commands from the app (orders, missions, production)
have no replay command, so no app session can be replayed; a played tactical
battle has no recorded command stream; and the sweep has run on Linux
x86_64 and Chromium 157 only (macOS, other browsers and other CPUs are
untested). This is not a claim of full interactive game parity.
