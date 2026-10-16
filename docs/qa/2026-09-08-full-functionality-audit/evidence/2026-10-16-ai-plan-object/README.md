# The AI's plan object, states 1 to 6: module 8, stage 4b (F-036)

Date: 2026-10-16.

Kind 3's plan object steps (`FUN_004adb70`) as the sector planner's state
5: its base, the regiments and squadrons of its held fleets, one idle
held fleet a cycle, and that fleet's people. States 7 to `0xe` (the
fleet's role, attacks and bombardment) come next (stage 4c). The reading
is in `ghidra/notes/ai-sector-plans.md` ("The plan object's step").

## Port (save v53)

- `crates/rebellion-core/src/ai_plan.rs`:
  - 1 `FUN_004af6d0`: the sector's systems lose the role marks
    `0x1f00000`;
  - 2 slot `+0x34`, `FUN_004af8a0`: keep the base while the side's, not
    blockaded and the headquarters when the sector has one (dropping
    `0x30000`), else drop it (`0x70000`); choose the sector's
    headquarters, else an own system (key `0x26`, order 1), else one of
    the nearest sector (`FUN_0041b2b0`), never another plan's base; the
    base takes `0x40000`;
  - 3 slot `+0x38`, `FUN_004afce0`: the first held fleet carrying
    regiments at an own system unloads its held ships' regiments there
    (Move `0x201`, `AIAction::MoveRegiments`; `FUN_0041acd0`,
    `FUN_0041ac80`);
  - 4 `FUN_004b0120`: the base's squadrons board the held fleet at the
    base, up to its room (`AIAction::MoveSquadronTeam`, new);
  - 5 `FUN_004b04c0`: the next held fleet without `0xf0003802`, the
    search going round the list once from `+0x2c`;
  - 6 `FUN_004b06b0`: the flagship's fleet stays or goes to `+0x3c`; else
    a fleet at an own system sets its people down there when one is the
    side's (`AIAction::MoveCharacters`, new).
- A planner's reset (`FUN_00477190`) gives up its base and the base's
  marks `0x70000`. `SectorPlanner` keeps the plan object's `+0x28` and
  `+0x2c`. The AI's modules get the troop transport (the cargo).
- The two new orders are validated (`FUN_00578d00`: each squadron class's
  move; each person the side's, free, aboard a fleet out of hyperspace at
  the target) and carried out by the integrator and the app
  (`squadrons::move_squadrons`, `missions::move_member`).

port:
- States 7 to `0xe` are the port's fleet heuristics, entered at 7 or `0xe`
  once a cycle; their end ends the plan's step. No held fleet has a role
  or a target yet, so state 1 only clears.
- A fleet's regiments and people are its cargo and characters, carried by
  its first ship's record; no movement planner holds anyone, and the port
  names no flagship.
- hyp: `FUN_005039d0` mode 1 gives the side's squadrons in the planet's
  order.

The test of the planner's cycle (`a_cycle_runs_state_by_state_and_reports`)
changed its expectation with this stage: state 5 now steps the plan
object 0 -> 1 -> 2 (a base) -> 3 -> 4 -> 5 -> `0xe` before the stand-in
(`FUN_004adb70`), six passes more; its assertions on the cycle and the
report are unchanged.

## Campaign (`playtest-plan-object.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`: every
event count equals stage 4a's (`identical_to_stage4a: true`). A temporary
trace (removed) showed the Alliance's first planner taking its
headquarters as base and its plan object reaching states 3, 4 and 6 each
cycle; its held fleet carries no regiments or people and has no hangar
room, and the base has no squadrons, so no state gives an order. Seeds 1,
2, 3, 7, 11, 13, 47 and 99 give none either: the port has no movement
planners to load the held fleets.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3745 | 0.5 ms | 0.7 ms | 2.0 ms |
| Fast | 60 s | 3750 | 0.5 ms | 1.2 ms | 2.3 ms |

No frame took over 50 ms. The AI's longest day took 20 ms (day 1 at Very
Slow, 600 passes). The Fast run reached day 151 (`2-fast.png`). The
console has 151 `[ai]` lines and 0 errors; `network.txt` lists 4
requests, all HTTP 200. Native GUI acceptance was not run.

## Gates

- `cargo test --workspace --no-fail-fast`: 2170 passed, 0 failed.
  - `ai_plan`: 17 tests (each state, the base's choice, keeping and loss,
    the ties by coin, the unloading, the squadrons' room and classes, the
    idle fleet search, the people and the flagship, the reset); `ai.rs`:
    the two orders' validation; the integrator: both orders carried out.
  - Each new test fails with its change neutralized (30 of 31 cases; the
    31st, the cap `min(room, v[25])`, is equivalent in the port, as
    `v[25]` counts every squadron of the planet).
- Seed-42 golden regenerated for save v53 (the plan objects' fields and
  bases): initial `v1:030a4e7e7f7902e9` -> `v1:a9d12433c6c44f1e`, final
  `v1:9bccfb22f8b4f1d5` -> `v1:a5c61ad2824e809c`. The replay equivalence
  check passes with 0 browser errors. WASM SHA-256
  `eb6cf508ca687fafdd4c1ca3d6998f8d322c0e760801677caac881ec12a3c5ed`.
- `cargo mutants --in-diff`: core 145 mutants, the first run missed 16;
  after the tie, mark and squadron tests six remain, all equivalent:
  - `choose_base`'s `kind & 0x3_0000 != 0` to `|`/`^`: clearing bits that
    are not set changes nothing;
  - `unload`'s coin (`< 5` to `==`, `>`, `<=`): the ships' order does not
    matter, as only the first ship's record carries the cargo;
  - `base_squadrons`' `values[23] > 0` to `>=`: a fleet without room
    takes nothing either way.
  Data: 6 mutants; the first run missed 2 (the applied flags), caught
  after the tests read them.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
