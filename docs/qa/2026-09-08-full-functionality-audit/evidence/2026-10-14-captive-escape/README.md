# Captive escape port (F-034)

Date: 2026-10-14. Runner: Claude Code, local Linux checkout. Source:
`ghidra/notes/captive-escape.md`.

## Change

`MissionSystem::advance_escapes` replaces `check_escapes`, which rolled every
captive every day on loyalty alone.

- Timer `0x38a` per captive: GNPRTB 2584 + rand(0..=2585) days after
  capture and between attempts (`FUN_004f1ba0`, `FUN_004ef750`,
  `FUN_005862a0`).
- ESCAPETB input: espionage + combat − the integer average combat of the
  captor's free, usable characters at the captive's system − the captor's
  regiments there (`FUN_00561990`, `FUN_0055e7e0`).
- A hit frees the captive with its home side and autoroutes it to the
  nearest unblockaded system of its side, then a system with its side's
  fleet, its blockaded system, a contested system, and an enemy system
  (`FUN_004f18e0`, `FUN_0054c7d0`, `FUN_005545e0`, `FUN_0058b060`). The trip
  takes `FUN_0055d8c0` days at the GNPRTB 1 default speed.
- `MissionState` keeps the timers and the escapees on the road. Save format
  v35; older saves are rejected.

## Results

| Check | Result |
|---|---|
| `cargo test --workspace` | all pass (core 751, data 128) |
| New tests | 14 in `missions.rs`, 1 in `integrator.rs`, 1 save round trip in `save.rs` |
| `cargo mutants` on `advance_escapes`, `escape_input`, `escape_destination` | 60 caught, 1 unviable, 0 missed |
| `cargo mutants` on `apply_escape_effects` | 3 caught |
| Clippy, touched crates | 8 warnings before and after, none in new code |
| rustfmt, touched files | no new diffs (pre-existing diffs unchanged) |
| `provenance-scan -- check` | 108 uncited, within baseline |
| Seed-42 golden | regenerated for this cause and save v35: `v1:18b0d4ae14e00774` -> `v1:501c8c959ff47345`; byte-identical across two recordings |
| `replay-gate` (`native-report.json`) | passed, tick 25 |
| Browser `?replay-check=seed42-v1` (`browser-report.json`) | passed; every field equals the native report |
| Browser requests and console | 4 of 4 HTTP 200; no errors |

The browser run used `cdp-replay.mjs` (headless Chromium over CDP) because
`agent-browser` is not installed here. The invalid-query, missing-pack and
normal-startup cases were not rerun. No interactive GUI check was made: the
escape has no new interface.

## Playtests

`playtest-captures-escapes.jsonl`: five dual-AI runs of 5000 days, seeds 1
to 5, `--release`. Seeds 3 and 5 capture 22 characters in all, and 22
escapes follow, between 124 and 4152 days after capture. Seed 5 recaptures
Drayson and Talon Karrde, who escape again. Seeds 1, 2 and 4 capture no one.

Hashes are in `SHA256SUMS`. `web/open-rebellion.wasm` is an unoptimized
build (`wasm-opt` is not installed).
