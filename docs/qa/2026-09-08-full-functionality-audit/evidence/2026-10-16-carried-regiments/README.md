# Module `0xb` sends carried regiments to a planner's base: module 8, stage 5i (F-036)

Date: 2026-10-16.

Stage 5h ported the planner's request and module 6's planned moves. It
only read module `0xb`'s fleet cycle state 4 (`FUN_004c9cb0`), which reads
the same request. This stage reads that function in full and ports it.
The reading is in `ghidra/notes/ai-production-modules.md` (module `0xb`,
"fleet cycle").

## Port

- `ai_module_b.rs`, `ModuleB::carry_to_base` (fleet cycle state 4), runs
  while controller `+4` has `0x30` (`sector_short` or `sector_invading`):
  1. It walks the planned sectors (`FUN_00419e90`: sector `+0x2c`
     `0x8000000`, order 2) in pick order. It takes the base (`+0x40`) of
     the first one whose planner (the sector record's `+0x38`) asks for
     regiments (`FUN_00476b70` > 0) and whose base is not blockaded
     (`+0x28` bit 2).
  2. Failing that, it takes the first pick's base, blockaded or not, if
     that planner asks (`0x4c9ef0`).
  3. It walks the side's fleets (`FUN_0041a4c0`: `+0x38` 5, forbidding 2,
     key `0xc`, order 1) in pick order. The first fleet that has free own
     ships (`FUN_0041ac80`: `0x24000`, forbidding `0x800`) and carries
     regiments gives them all: Move `0x201` to the base.
  4. `+0x54` is left at the next fleet in the walk.
- `ai_module1::Inputs` gains `transport`, the regiments each fleet
  carries. The AI passes the tick's `TroopTransportState`.

Save format: unchanged at v57.

port:
- A fleet's regiments are its cargo, not a ship's. So a fleet gives all
  its cargo when any of its ships passes the mask.

## Tests that changed with this stage

None of the old assertions changed. Module `0xb`'s test helper `with_io`
takes the transport state and module 8's planners.

## Campaigns (`playtest-carried-regiments.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, compared with the
stage 5h build:
- Seeds 42, 2, 3, 7, 13 and 47 give the same event stream, apart from
  timing.
- Seed 1 differs from day 22 on. That day module `0xb` sends one carried
  regiment to Sullust, and the campaign diverges from there. The planets
  changing hands stay at 8.

## Browser

The run used the release build with `agent-browser` (headless Chromium,
640 by 480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.
Each frame's work time was measured:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3600 | 0.5 ms | 0.7 ms | 23.5 ms |
| Fast | 60 s | 3601 | 0.6 ms | 1.5 ms | 3.0 ms |

- No frame took over 50 ms.
- The AI's longest day took 19 ms.
- The Fast run reached day 152.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- The Fleet window gate (`fleet-window.mjs`, Arch Chromium 153 through a
  temporary Linux manifest): all 46 cases pass.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2239 passed, 0 failed.
  - The tests in `ai_module_b.rs` check:
    - the order and the cycle's end;
    - each controller bit, and none without them;
    - the sector bit, a planner asking for nothing, and a planner of
      another id;
    - ships held by a planner, and a fleet without regiments;
    - the first unblockaded base, and the first pick's base as the
      fallback, which still needs a planner that asks;
    - the fleets by their key, the other side's fleet passed over, and
      `+0x54`.
  - 15 cases fail with their change neutralized: state 4, both bits, the
    sector bit, the request, the planner lookup, the blockade, the
    fallback and its end, the sector order, the fleet order and mask, the
    ship mask, `+0x54`, and the first fleet only.
- `cargo test -p rebellion-data --tests -- --include-ignored`: the replay
  goldens pass unchanged. Two tests fail, both also failing before this
  stage: `fixed_fleet_tables_only_target_special_systems` and
  `telemetry_coverage`.
- Seed-42 golden: `v1:a8343f6c6630924e`, final `v1:e523a780152b284e`,
  both unchanged. The replay equivalence check passes with 0 browser
  errors. WASM SHA-256
  `85b632d4d8af5f3764fa4b4de47b2390476144527ecf08b1fd817efca581daa8`.
- `cargo mutants --in-diff` (core): 23 mutants, 21 caught, 1 unviable, 1
  missed. The missed one is equivalent: `SHIP_FREE | SHIP_OWN` to `^`,
  since the two bits are disjoint.
- `provenance-scan check`: 102 uncited items, within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
