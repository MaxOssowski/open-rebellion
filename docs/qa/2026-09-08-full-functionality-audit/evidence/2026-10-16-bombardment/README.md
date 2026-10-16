# Planetary bombardment: module 8, stages 4d and 4e (F-032, F-036)

Date: 2026-10-16.

Stage 4d read the Fleet menu's Planetary Bombardment orders and how the
game resolves them (`ghidra/notes/bombardment-order.md`, commit
`af931fb0`). Stage 4e ports them: `bombardment.rs` is rebuilt on the
recovered functions, the AI's plan objects order bombardments, and the
automatic bombardment after a battle, which the original does not have, is
gone (the user chose to remove it).

## Port (save v55)

- `crates/rebellion-core/src/bombardment.rs` (`resolve`, `apply`,
  `BombardEnv`), `FUN_0058d770`'s four phases:
  - 1 `FUN_0058d990`: with Destroy System `0x223` a Death Star among the
    bombarding ships destroys the system (the caller's Death Star
    cleanup);
  - 2 `FUN_0058dc10`, `FUN_0058d540`: each ion cannon (DEFFACSD family
    `0x22`) and battery (`0x23`) hits a drawn ship with `(general / G0x600
    + 1) * attack`; the ship's shields (`FUN_00501160`) take the ion hits
    then the battery hits; the battery hits past them damage the hull, up
    to what it has left; ion hits past them stop the ship bombarding
    (`FUN_00500f00`); no ship left stops the bombardment;
  - 3 `FUN_0058df60`: firepower (each ship's `bombardment_modifier` less
    its share of hull damage, each squadron's FIGHTSD `+0xcc`, times
    `(admiral / G0x601 + 1)`) less the shields of the system's side
    (`0x24`) gives the shots; with `0x220` a collateral roll at the first
    civilian facility (`FUN_0058d080`); each shot draws a target among
    those counted at the start and destroys it when its bombardment
    defense is below a draw of G`0x602`..G`0x603` (1 to 10); the shots
    stop when none is left;
  - 4 `FUN_0058e320`: civilian damage (a manufacturing or production
    facility, an energy or raw unit) moves the attacker's support by
    G`0x1e19` (-20, `uprising::apply_support_change`).
  - Targets by kind (`FUN_0058d800`, `FUN_0058cba0`): `0x220` the
    regiments, defense facilities and the Alliance's headquarters; `0x221`
    the manufacturing and production facilities; `0x222` and `0x223` all
    of them, an energy and a raw material unit; each of the system's side.
  - The headquarters (ALLFACSD 1, bombardment defense 9 from the record's
    last word, now loaded) is a target only at the Alliance's HQ system;
    its loss clears `System::is_headquarters`, the Empire's HQ objective.
- `AIAction::Bombard`: a plan object's bombarder at its target
  (`FUN_004b1070` -> `FUN_004b1330`: `0x223` with fleet `0x10` and system
  `v[50]` above 0, else `FUN_004ae320`'s code); the order stops the walk,
  `SectorPlanner::plan_walk` (`+0x30`) naming the next fleet. Validated by
  the orders' enabled check (`FUN_00553770`, `FUN_00582440`) and, port:,
  the fleets in orbit, out of hyperspace, at a system not the side's.
- The integrator and the app carry it out through `BombardEnv`; a
  Destroy System's planet goes through the Death Star's `PlanetDestroyed`
  (simulation step 11; the app's `destroy_planet`, shared with the
  superlaser). The player's Bombard button orders General Bombardment
  until the sub-menu (stage 4f). Telemetry: `bombardment` with the kind,
  shots, targets destroyed, ships hit, civilian damage and the HQ.
- Removed: `BombardmentSystem::resolve_bombardment` (the transit misread),
  `VictorySystem::apply_headquarters_bombardment`, the battle follow-ups
  in `simulation.rs` and the app (`tactical_flow::apply_automatic_bombardment`).

port:
- The draws come from a `SplitMix64` stream seeded by one roll of the
  tick (the app: its generator); the original has one game generator.
- No officer ranks: the general's and the admiral's factors are 1.
- hyp: the manufacturing facilities precede the production ones in the
  system's list; a ship's `+0x24 & 0x30` is 0; an unset packed word reads
  as a shield nibble of 0; a Death Star's destruction leaves no battery
  to fire.
- Not ported: the minor characters' hits on destruction (the Death Star
  cleanup kills those aboard its fleets), the sector and galaxy support
  events (`FUN_00518b20`), the bombarded bit (`+0x88` `0x40`).

Tests that changed with this stage:
- `simulation.rs`: `decisive_imperial_bombardment_destroys_the_alliance_hq_facility`
  and `unopposed_imperial_invasion_bombards_before_occupying_alliance_hq`
  asserted the invented automatic bombardment; they now assert, on the
  same campaigns, that no bombardment happens and the HQ facility stands
  (renamed `a_decisive_imperial_victory_leaves_the_alliance_hq_facility_standing`,
  `an_unopposed_imperial_invasion_occupies_the_alliance_hq_without_bombarding`).
- `victory.rs`: the three tests of `apply_headquarters_bombardment` now
  destroy the headquarters through `bombardment::apply`; the refusals they
  covered are the target rules in `bombardment.rs`'s tests.
- `bombardment.rs`: the old formula's tests went with it.
- `ai_plan.rs`: the step's test helper now carries an order with the end
  (`PlanStep::Ended(Option<AIAction>)`).

## Campaign (`playtest-bombardment.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`: no
bombardment (as in stage 4c once the stand-in had gone); the tick takes
one roll more, so later draws shift. In 19 campaign runs over 18 seeds
(600 and 900 days, a temporary trace) no plan object binds a role, so the
AI never orders a bombardment; with the automatic one gone nothing
bombards in these campaigns.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign, 60 s at Very Slow then 60 s at Fast. This run measured
the `requestAnimationFrame` intervals, not the frame work times of the
earlier stages' script (lost with its session directory): the longest
interval was 16.8 ms in both runs, so no frame took over 50 ms; the Fast
run's frame count is not reported (it does not match its time). The AI's
longest day took 18 ms (day 1 at Very Slow). The Fast run reached day 152
(`2-fast.png`). The console has 152 `[ai]` lines and 0 errors;
`network.txt` lists 4 requests, all HTTP 200. Native GUI acceptance was
not run.

## Gates

- `cargo test --workspace --no-fail-fast`: all pass, 0 failed (1205 in
  `rebellion-core`).
  - `bombardment.rs`: 13 tests (the targets by kind, the firepower and
    shields, the ion cannons and the batteries with the shield
    recomputation, the shots' draws and rolls, the collateral roll, energy
    and raw units, the Death Star, the headquarters, the draws' stream, the
    tick's bombardments); `ai_plan.rs`: the bombarder's order, Destroy
    System, the walk's cursor; `ai.rs`: the order's validation; the
    integrator: a bombardment carried out with its telemetry;
    `simulation.rs`: a destroyed system as `PlanetDestroyed`; an ignored
    data test: the headquarters' defense 9 (run, passes).
  - Each new behaviour fails with its change neutralized (48 cases).
- Seed-42 golden regenerated for save v55 and the bombardments' roll
  (`v1:dbb908594619dd4b` -> `v1:b160883b9ed0c807`, final
  `v1:58f498ebb709e88b` -> `v1:f5933994514e98bc`); two recordings
  identical. The replay equivalence check passes with 0 browser errors.
  WASM SHA-256
  `1962dde522fa6b7d8600a05a40a3f2023da02bcd3b97128a95ae4f7c43917851`.
- `cargo mutants --in-diff`: core 244 mutants; the first run missed 36,
  the second after the new tests 12, all equivalent:
  - `settle_role`'s `& ROLES` and `& ROLE_BOMBARD` to `|`/`^`: a fleet
    without a role has no target, and kind 3 binds no other role;
  - `upto`'s negative branch without its minus: it runs only with no ship
    or no target to index;
  - `excess`'s `>` to `>=` and the shots' `firepower > shields` to `>=`:
    equal values give 0 either way;
  - the leadership factors' `/` to `%` or `*`: the port has no officer, so
    the leadership is 0;
  - the firepower's `b.fleet == fleet && alive(b)` to `||`: a lost ship's
    bombardment is 0.
  Data: 15 mutants, 4 missed: the bombardments' seed (`*` to `/` or `+`;
  any seed is valid and the replay does not bombard) and two that need the
  original data (`load_game_data_with_options`, the headquarters'
  defense), which the ignored data test catches.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
