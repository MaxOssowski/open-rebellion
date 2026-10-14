# The AI's cadence in the game loop (F-036 follow-up)

Date: 2026-10-14. `agent-browser` 0.38.1, headless Chromium, 640 by 480.
The release site `web/` (`./scripts/build-wasm.sh`) is served by
`python3 -m http.server`. A driver calls `wasm_exports.frame()` every 16 ms
and times each call.

This covers the checks left open in `../2026-10-14-ai-cadence/`: the app's
own loop (`main.rs`), the cost of the Very Slow day in the browser, and the
repeated orders.

## The app's loop

`main.rs` now logs one line per day:
`[ai] day=N speed=S passes=P orders=O second_orders=O2 ms=T`, the day's AI
work and its time. The run:

1. Main menu (`1-main-menu.png`). The Empire monitor starts "Galactic
   Empire, Easy, Small Galaxy, Standard Game" (console `[campaign]`).
2. A right-click in the day readout (`(499,19)-(580,31)`, `FUN_00422ce0`)
   opens the Game Speed menu (`2-speed-menu.png`).
3. Fast, 20 s: days 1 to 33 at `passes=4`. The AI spent its 10 orders on
   day 1, gave 8 on day 2, and then 0 to 2 a day. Frames: 1245, median
   0.4 ms, longest 9.1 ms. At day 33 a battle at Bothawui opened the
   battle window and the run stopped there.
4. A new campaign at Very Slow, 135 s: `passes=600`. Day 1 took 1.0 ms
   (the 10 orders were spent early, so the remaining passes return at
   once), day 2 took 18.0 ms (8 orders, every pass ran). Frames: 8434,
   median 0.4 ms, longest 18.7 ms, none over 50 ms. `3-very-slow-day-2.png`
   shows the readout at day 2 with the speed bar.

At Very Slow a day lasts 60 s, so the cost is one frame of about 18 ms a
minute this early in a campaign. The release build measured about 28 ms a
day natively over 600 days with two AIs
(`../2026-10-14-ai-cadence/playtest-speeds.json`). No optimisation is
needed now.

`console.txt` holds the session's console: 77 messages, 0 errors, 39 `[ai]`
lines. `network.txt` lists 12 requests, all HTTP 200.

Native: not run. The session is Hyprland (Wayland) without `xdotool`, and
`cua-driver` is the macOS route in `agent_docs/agent-tooling.md`. The app's
loop is the same code on both targets.

## The repeated orders

A seed-42 dual-AI playtest with a temporary print of non-1 end codes (since
removed):

- Sabotage of `0x3c000002` at Coruscant (days 54 to 71, daily): each
  mission was detected and ended with code 3, or 4 past phase 4
  (`mission_detection.rs`, slot `+0x1dc`, record `can_resign`). The team
  resigned free and the AI ordered it again. The original planner does the
  same: it counts failed results against slot `+0x24` (`FUN_00480df0`,
  1000) before it gives up. The daily cadence only makes the retries more
  frequent. Not a defect.
- Recruitment of Palpatine and Vader at Coruscant (every 5 to 30 days):
  successive missions, each one resolved (`mission_resolved`), not
  failures.

## Gates

- `cargo test -p rebellion-app`: 74 passed.
- `scripts/check-replay-equivalence.py --skip-build --json` passes on the
  rebuilt WASM: nine checkpoints, final `v1:4225657ae5e05393`, 0 browser
  errors. WASM SHA-256
  `d04a51f7c2890a2b813dc3f96afd891b722bd3721b5a034ce24ec9c6e73ef29e`.
- `cargo mutants --in-diff`: 1 mutant, `main` in `main.rs`, missed: no test
  runs the app's loop.
- rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
