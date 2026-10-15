# The AI's sector planner step: module 8, stage 4a (F-036)

Date: 2026-10-15.

The sector planner's own step (`FUN_004774c0`): it holds capital ships
and fleets for its plan, weighs them against what the plan wants, takes
or gives ships with Create Fleet `0x270`, and reports to the strategy.
The plan object's own step (`FUN_004adb70`, 14 states) comes next
(stage 4b). The reading is in `ghidra/notes/ai-sector-plans.md` ("The
planner's step").

## Port (save v52)

- `crates/rebellion-core/src/ai_planner.rs`:
  - holding: `FUN_00478e00` (a ship's record loses `0x20000`, names the
    planner and plan; its values add to the counts), `FUN_004790d0`,
    `FUN_00478fd0`, `FUN_004791d0`; after an order the lists are checked
    against the records (`FUN_00477810`, `FUN_00477950`);
  - the step's states: 1 damaged ships off (`FUN_00477a40`); 2 the counts
    from the held fleets (`FUN_00477cb0`) less what the plan wants
    (`FUN_004adaf0`, kind 3's `FUN_004b4920`); 3 take free ships of the
    nearest fleets (`FUN_004782a0`), else a later plan's
    (`FUN_00478690`), mark the short counts, else give one spare ship
    back (`FUN_00478aa0`), each by Create Fleet to `FUN_00479220`'s
    system; 4 the mission and movement requests (kind 3's
    `FUN_004b7150`, `FUN_004b1800`); 6 the flagship, the report (kind
    3's `FUN_004b3830`, `FUN_00479a00`);
  - kind 3's slots `+0x50`, `+0x20`, `+0x18`, `+0x1c`.
- `AIState::fleet_holds`: the fleet records' kept bits (`0xf000000c`),
  `+0x30`, `+0x34`. `AIAction::CreateFleet` gains its target (order
  `+0x34`): in place through `fleet_join::create_fleet`, elsewhere through
  `move_ships_to_system` (hyp: the order's move command).
- Module 8's state 4 runs the step; an order completes the module and
  raises its priority while the cycle goes on.

port:
- State 5, the plan object's step, is still the port's fleet heuristics,
  decided once a cycle; planners of kinds other than 3 run them for the
  whole cycle.
- No mission or movement planners: kind 3 asks for Recon or Espionage
  and movement 3, and no request is served; the flagship and `+0x3c`
  stay clear.
- hyp: the Create Fleet targets of `FUN_00477a40` (a list node read as
  an id) and the origin of `FUN_004782a0`/`FUN_00478aa0` (an unset
  local) are not ids; the port uses the ships' system. Controller
  `+0x164`/`+0x168` read as 0.

Four scheduler tests changed expectations with this stage: a kind 3
planner's cycle goes on after the stand-in's last order (state 6), so
module 8 keeps its priority one pass more and ends the cycle then
without an order (`FUN_004774c0`, `FUN_004cf630`). Their scheduling
assertions are unchanged.

## Campaign (`playtest-planner-step.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`, with a
temporary trace (removed):
- The Alliance's kind 3 planner gives a ship back on day 34 (Create
  Fleet to its system); the strategy then grows to two plans, shrinks on
  day 384 (two planners gave up), grows again on day 448. In all, three
  Create Fleet orders.
- At the end the first planner holds one ship and one fleet with no
  count short (report `0xc1e0000`); the second, for a sector that wants
  628 and 853 of attack, finds no free ship (gave up, `0xc030000`).
- The Empire's planners hold kind 4 and 2 plans, which run the
  heuristics.
- The run's events change against stage 3 (the planners' draws and
  holds change the AI): 753 AI actions against 535, 12 space battles
  against 4, 47 ships repaired against none.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3747 | 0.5 ms | 0.7 ms | 24.8 ms |
| Fast | 60 s | 3743 | 0.5 ms | 0.7 ms | 1.5 ms |

No frame took over 50 ms. The AI's longest day took 20 ms (day 1 at
Very Slow, 600 passes). The Fast run reached day 151 (`2-fast.png`):
the earlier stages stopped on the battle screen at Bothawui, which this
run's campaign does not reach. The console has 151 `[ai]` lines and 0
errors; `network.txt` lists 4 requests, all HTTP 200. Native GUI
acceptance was not run.

## Gates

- `cargo test --workspace --no-fail-fast`: 2149 passed, 0 failed.
  - `ai_planner`: 32 tests (holding, the lists, each state, the needs
    and spares by table, the ties by coin, the destination); `ai.rs`: a
    planner sending its damaged ships off, a planner of another kind
    ending its cycle; the four scheduler tests above.
  - Each new test fails with its change neutralized.
- Seed-42 golden regenerated for save v52 and the planners' draws:
  initial `v1:1ec77e5da40c2f8f` -> `v1:030a4e7e7f7902e9`, final
  `v1:73074bf4b25f5bf9` -> `v1:9bccfb22f8b4f1d5`. The replay equivalence
  check passes with 0 browser errors. WASM SHA-256
  `b11b3a8ef05c8caf6ad54b8d7d436433dc07c4e7a8c3a2e27055054b7f078189`.
- `cargo mutants --in-diff` (`ai_planner.rs` whole and the changed lines
  of `ai.rs`, `ai_records.rs`, `ai_module8.rs`): 395 mutants; the first
  run missed 99, the second 19 after the table tests; a third run over
  the functions still open (`balance`, `meets`, `spare`,
  `random_system`) caught all 100 viable ones. Six are equivalent:
  - `edit_fleet`'s `& FLEET_KEPT` to `|`/`^` and `apply_fleet_holds`'
    `|` to `^`: the hold's bits are masked when applied, and the masked
    parts are disjoint;
  - `release_ship`'s `| SHIP_FREE` to `^`: a held ship's record lacks
    `0x20000`;
  - `split_damaged`'s `flags & 0x3000` to `|`/`^`: in the port a fleet
    record has `0x1000`/`0x2000` exactly when a ship is damaged, which
    the ship test checks again.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
