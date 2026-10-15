# Planet squadrons in battle (F-033, F-021)

Date: 2026-10-15. Stage C of the planet squadrons
(`../2026-10-15-planet-squadrons/`).

## What the original does

Read in `ghidra/notes/planet-squadrons.md`, "Battle":

- A system's BattlePending bit (`+0x88` `0x1000`, `FUN_0050a900`) is set by
  `FUN_0050b8e0` when both sides are present (`FUN_00509710`): an active
  fleet or an active squadron on the planet.
- The battle manager `FUN_00516d50` takes a system with BattlePending and
  sets its Battle bit (`+0x10`, `FUN_0050a580(1)`). A fleet entering a
  system where the other side has only squadrons on the planet therefore
  meets a battle (manual p. 141).
- At the end `FUN_00516ef0` counts each side's fleets and planet squadrons,
  sets the loser's fleets and planet squadrons autorouting (`FUN_004f78e0`)
  and hands the winner's planet squadrons to `FUN_00556710`; `FUN_00517270`
  records the side left (`FUN_0050a040`).
- untraced: how the battle itself takes the planet's squadrons.

Found on the way: the "space combat" pipeline of `space-combat.md`
(`FUN_005457f0` → `FUN_00549910`) runs only for story event ids
(`FUN_005438a0`); it is the mission manager's. The note now says so.

## Port

- `squadrons::sides_present`: the presence rule. The headless step
  (`simulation.rs`) and the app's loop (`main.rs`) start a battle where both
  sides are present, where they used to need a fleet of each side.
- `squadrons::scramble` / `recover` (`Sortie`): for the battle's length each
  side's planet squadrons join its first fleet in orbit, or a group of their
  own when it has none; after it the survivors land again and a group is
  removed. Both battle paths do this: the automatic one, and the player's
  tactical battle, whose sorties land when it ends (Simulate, or the return
  to the galaxy view). A group is gone before bombardment and landing, so it
  does neither; the headless step now skips a winner fleet that no longer
  exists.

port:
- The original's battle takes the squadrons from the planet; the port's
  combat is fleet against fleet (F-033), so they fight inside a fleet. The
  squadrons aboard keep no identity, so the planet takes back its own first.
- The loser's autorouting is not ported: the port's battles end when a side
  is destroyed or a round changes nothing.

No save format change: a sortie lives only within a battle, and a tactical
battle cannot be saved (`GameOptionsOrigin::TacticalBattle`).

## Campaign (`playtest-battle.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`: no squadron
stands on a planet in this campaign, so the orders by side and type and the
one space battle are as before.

## Browser

### The squadron battle (`fleet-window-run.json`)

A new fixture scenario, `SquadronBattle` (65, code 66): the `FleetLoad`
world with two of the other side's squadrons on the primary planet, under
the player's fleet. The case `squadron-battle`, on both sides:

1. Before the clock runs: two enemy squadrons on the planet, no enemy fleet
   (`3-*-squadron-battle-before.png`).
2. Fast: the battle alert opens ("Battle at Akrit'tar"), the squadrons are
   up as one enemy group (`3-*-squadron-battle.png`).
3. Simulate: the battle ends in a draw (the fixture's squadrons and
   transport carry no weapons); the group is gone and both squadrons stand
   on the planet again (`3-*-squadron-battle-after.png`).

The whole fleet-window family ran, 44 cases, all passing, with 0 console
errors in the new case. As in stage 1, the run used the system Chromium
153.0.8010.52 through an untracked copy of the harness without the version
pin (the pinned macOS Chrome for Testing 151 is not on this machine).

### Release build

`agent-browser` (headless Chromium, 640 by 480), a new Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3746 | 0.5 ms | 0.6 ms | 17.9 ms |
| Fast | 60 s | 3744 | 0.4 ms | 0.6 ms | 2.0 ms |

No frame took over 50 ms. The AI's longest day at Fast took 2 ms. The
console has 32 `[ai]` lines and 0 errors; `network.txt` lists 4 requests,
all HTTP 200.

Native GUI acceptance was not run.

## Gates

- `cargo test --workspace --no-fail-fast`: 2019 passed, 0 failed.
  - `squadrons.rs`: three tests for presence, the sortie and its recovery,
    and a destroyed or emptied sortie.
  - `simulation.rs`:
    `an_enemy_planet_s_squadrons_meet_an_arriving_fleet_in_battle` fails
    with the old fleets-only trigger, and caught the bombardment of a
    removed group (a panic) before the winner check.
  - The fixture: `the_squadron_battle_scenario_puts_two_enemy_squadrons_under_the_fleet`;
    the object-menu scenario list adds `SquadronBattle`.
- Seed-42 golden unchanged: initial `v1:d33088556c4445a3`, final
  `v1:9ff9c886bcd2e454`. The replay equivalence check
  (`replay-equivalence.json`) passes with 0 browser errors. Release WASM
  SHA-256 `299ac2fac17a1650a423cb0e24131507593d50b5b4e9f47330a60ff0f36e685d`.
- `cargo mutants --in-diff` (`squadrons.rs`, `simulation.rs`): 29 mutants,
  24 caught, 3 missed, 2 unviable. One guard was redundant and is gone; a
  test covers an emptied entry. A retest of `scramble`, `recover` and
  `sides_present` caught all 22 viable mutants.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in the changed lines. rustfmt: no findings in the
  changed lines.

Hashes are in `SHA256SUMS`.
