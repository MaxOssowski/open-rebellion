# Planetary Assault: module 8, stage 5f (F-006, F-036)

Date: 2026-10-16.

Stage 5e read the Fleet menu's Planetary Assault order and its
resolution (`ghidra/notes/planetary-assault.md`, commit `bb449f5f`). This
stage ports the assault. It also removes the port's invented invasion
(the user's choice, "Całość, wiernie"):
- the automatic landing on another side's planet;
- the automatic ground battle;
- the app's interactive ground-combat screen.

## Port

- `crates/rebellion-core/src/assault.rs`, `FUN_0058c120`'s phases:
  - 1, the counts: the attacking regiments aboard the order's fleets, and
    the defending regiments of the system's side;
  - 2, `FUN_0058c3c0`: each defense facility fires with chance attack /
    G`0x200` percent and destroys a drawn landing regiment;
  - 3, `FUN_0058bec0`: each attacking regiment fights a drawn defender in a
    duel, with x = a draw of 0..G`0x202` + TROOPSD attack − defense. The
    attacker falls at x ≤ G`0x203`, the defender at x ≥ G`0x204`;
  - 4, `FUN_0058c580`: each duel has a G`0x205` percent chance to destroy a
    drawn facility, energy unit or raw unit;
  - 5, `FUN_0058c810`: with no defender left, up to ceil(G`0x1e51` /
    −G`0x1e52`) = 6 attacking regiments land.
- `apply` carries the outcome out, `order_enabled` checks whether the order
  is enabled (`FUN_00581ae0`), and `BombardEnv::assault` takes its draws
  from the tick's stream.
- `AIAction::Assault`: validated by `order_enabled` and carried out by the
  integrator and the app. Telemetry goes to `combat_ground`, and the app
  writes an Assault Summary line. No plan object orders it yet; the kind 4
  plan comes later.
- The simulation's step 3b and the app's `tactical_flow::land_and_occupy`
  work the same way:
  - regiments carried to a system land only on their side's planet that
    stands and is not blockaded (`FUN_005073d0`);
  - one side's regiments alone take a planet (port);
  - two sides' regiments stand together.
- Removed:
  - `CombatSystem::resolve_ground`, `GroundCombatResult`,
    `TroopDamageEvent` and the integrator's ground-combat helpers;
  - the simulation's surface battle;
  - the app's landing, post-battle and continuing ground routes
    (`resolve_post_battle`, `resolve_ground_campaign`);
  - `GameMode::GroundCombat` and `rebellion-render`'s `ground_combat.rs`.
- `BlockadeSystem::system_is_blockaded` is now public.

port:
- There are no officer ranks, so the generals' leadership terms are 0.
- A fleet's regiments are its cargo, and the walk goes through each fleet
  in turn.
- The order's member target test (`FUN_0053f150`) is not read. In its
  place, each fleet must orbit the system out of hyperspace, and the
  system must stand, not be the side's, and have at most one shield (the
  manual's fully-shielded rule).
- hyp: phase 2 fires every defense facility of the system.
- hyp: phase 4 draws from the defenses, then the manufacturing, then the
  production facilities, of any side.

Save format: unchanged at v56. The seed-42 golden is unchanged, because no
regiment lands on another side's planet in its first 25 days.

## Tests that changed with this stage

- `simulation.rs`: three tests asserted the invented invasion. On the same
  campaigns they now assert the original's behaviour:
  - `surface_battle_continues_on_later_tick_after_transport_empties` is
    now `regiments_of_both_sides_stand_together_without_an_assault`: no
    battle and no losses, the Alliance does not take the planet (its
    original assertion), and later each side's regiment alone takes it;
  - `an_unopposed_imperial_invasion_occupies_the_alliance_hq_without_bombarding`
    is now `an_imperial_transport_at_the_alliance_hq_keeps_its_regiment_aboard`;
  - `troop_transport_arrives_lands_occupies_and_captures` is now
    `a_transport_at_the_other_side_s_system_keeps_its_regiment_aboard`.
- `combat.rs`: the eight `resolve_ground` tests went with it.
- `main.rs`: `tactical_ground_results_persist_survivor_damage_and_remove_losses`
  went with `apply_tactical_ground_strengths`.
- `tactical_flow.rs`: three post-battle ground tests are replaced by
  `land_and_occupy`'s tests.
- `ai_planner.rs` and other existing tests are unchanged.
- The interface fixture's loading scenarios give the empty target planet
  60 energy. Without regiments, a system keeps its holder only with energy
  at GNPRTB 7760 (`economy::resolve_system_control`, `FUN_0050b610`). The
  Fleet window gate's `move-and-land` case, a regiment landing on the
  player's planet, depended on the removed landing that ignored the
  holder.

## Campaigns (`playtest-assault.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, compared with the
stage 5c build:
- Seeds 42 and 47 are unchanged: no regiment was carried to another side's
  planet.
- In seeds 1, 2, 3, 7 and 13 the planets that changed hands fell from 9,
  10, 14, 14 and 12 to 7, 9, 11, 10 and 11. The AI took them by landing
  the regiments its heuristics carry. It orders no assault until the kind
  4 plan is ported, so it no longer takes the other side's planets that
  way.
- No `combat_ground` event happens in either build in these campaigns.

## Browser

The run used the release build with `agent-browser` (headless Chromium,
640 by 480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.
Each frame's work time was measured:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3600 | 0.6 ms | 0.8 ms | 23.5 ms |
| Fast | 60 s | 3601 | 0.6 ms | 1.4 ms | 3.2 ms |

- No frame took over 50 ms.
- The AI's longest day took 19 ms (day 1).
- The Fast run reached day 152 (`2-fast.png`).
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

Interface gates, run under Arch Chromium 153 through a temporary Linux
manifest (the pinned macOS build is not installed here):
- `fleet-window.mjs`: all 46 cases pass, both sides, including
  `move-and-land` and the pixel comparisons.
- `fleet-move.mjs` passes except `confirmed-move`. That case fails in its
  setup's map probe, as it did before this stage.

## Gates

- `cargo test --workspace --no-fail-fast`: 2219 passed, 0 failed.
  - `assault.rs` has nine tests:
    - the duels and who falls;
    - the landing and its limit;
    - the defenses' fire;
    - the collateral damage, and each draw's range;
    - an energy or raw unit taken once;
    - no garrison base;
    - the order's enabled check;
    - the env.
  - `ai.rs`: the assault's validation. The integrator: an assault carried
    out with its telemetry. `simulation.rs`: carried regiments land only
    on their side's standing planet. `tactical_flow.rs`: the same, the
    held fleet, and the occupation.
  - Each new behaviour fails with its change neutralized (11 cases).
- `cargo test -p rebellion-data --tests -- --include-ignored`: the replay
  goldens pass. The data test `fixed_fleet_tables_only_target_special_systems`
  fails, as it did before stage 5c.
- `cargo mutants --in-diff`:
  - First run: 198 mutants, 34 missed. The script clamps each draw to its
    range, so mutants that only change a range survived.
  - Second run, on the affected functions after the range and edge tests:
    149 mutants, 10 missed, 1 timeout. One more test then catches the
    energy count.
  - The remaining misses are equivalent or not covered by a headless test:
    - the generals' leadership terms (`/` to `%` or `*`, `+` to `-`): the
      port's leadership is 0;
    - `upto`'s negative branch: no count reaches it;
    - phase 3's `attackers && defenders` to `||`: the loop stops on either;
    - phase 4's raw-unit branch at `>=`: unreachable with no energy and no
      raw;
    - the simulation's Alliance-alone occupation arm: masked by the
      economy's troop-based control (`FUN_0050b610`), which runs earlier in
      the tick. The app's `land_and_occupy` tests cover it;
    - `main`'s `apply_ai_actions`: the app loop, with no headless test.
- `provenance-scan check`: 102 uncited items, within the baseline (one
  fewer).
- Clippy (pedantic) and rustfmt: no findings in the changed lines.
- The replay equivalence check passes with 0 browser errors. Golden
  `v1:fb57b4045cb49e83`, final `v1:d62b35fe73dd4203`, both unchanged.
  WASM SHA-256
  `3b70109985f2ef1fc871c03167971ed087abca8263576925eef945ae1347cf66`.

Hashes are in `SHA256SUMS`.
