# Research as missions (F-040, F-037)

Date: 2026-10-14. `agent-browser` 0.38.1, headless Chromium 153, 640 by 480;
driver `mission-flow.py 63 alliance,empire kind=1`.

Fixture `ResearchTargeting` (code 64) is `MissionTargeting` with the target
system (F'tral) held by the player at full support, the player's shipyard
there (MANFACSD `0x28000001`), the side's recruit pool empty, and the agent
a ship designer (ship design 40, troop training and facility design 0).
`kind=1` opens the dialog's drop-down ("missions" button), scrolls one kind
on and closes it before Begin.

| Side | Mission list | Picked | After Begin |
|---|---|---|---|
| Alliance | Espionage, Ship Design Research | Ship Design Research | ShipDesignResearch active |
| Empire | Espionage, Ship Design Research | Ship Design Research | ShipDesignResearch active |

Espionage lists because its record admits an own-side target. Full support
ends Diplomacy (`FUN_00573ee0`) and the empty pool Recruitment
(`FUN_0056b370`); the agent's zero troop and facility skills refuse those
trees at creation (`FUN_0056c820`), and the target has no training facility
or construction yard (`FUN_0056c510`, end `0xc`).

4 of 4 requests HTTP 200, 0 console errors on each side. Screenshots
inspected: `*/4c-kind-picked.png` shows the GOKRES Ship Design Research icon
(TEXTSTRA 11296) for each side, `empire/4b-kind-list.png` the open
drop-down. Fixture site WASM `open-rebellion-test.wasm` SHA-256
`b56a8293cb4616046eadf6eba97b13c7876d08150b025d7d8d024d3ddaad86df`.

## Headless campaign

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai` (release
build); `playtest-research.jsonl` keeps the research dispatches, results and
level-ups. The Empire AI sent Bevel Lemelisk to Coruscant on day 28; its
repeating mission resolved 37 times (31 successes) and the Ship tree reached
levels 1, 2, 3 and 4 on days 74, 205, 328 and 475. The Alliance AI sent
four Troop Training missions (11 results, 5 successes) and reached Troop
level 1 on day 542. Neither AI ordered Facility Design Research in this run;
why is not examined here.

## Gates

- `cargo test --workspace`: 1732 passed, 0 failed. The ignored seed-42
  replay passes with its golden regenerated for save v36 and research as
  missions: `v1:ee7a9a22f312f0e0` -> `v1:398db20957865176`.
  `scripts/check-replay-equivalence.py --skip-build --json` passes: native
  and WASM agree on all nine checkpoints, 4 requests, 0 browser errors.
- `cargo mutants --in-diff`: rebellion-core 105 mutants, 94 caught,
  11 unviable; rebellion-data 11, 9 caught, 2 unviable; rebellion-render 6,
  5 caught. The one survivor is `mission_kind_display` in the egui Missions
  panel, drawing code no unit test renders. rebellion-app 22: the survivors
  are `main`, `apply_panel_action`, `apply_ai_actions`, `apply_mission_result`
  in `main.rs` and the WASM bridge `emit_mission_flow`, which no native test
  reaches; the fixture's popularity mutant is caught after a tightened test.
- `provenance-scan check`: 104 uncited items, all within the baseline.

Hashes are in `SHA256SUMS`.
