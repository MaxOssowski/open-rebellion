# How the AI is scheduled

Read 2026-11-20 from the typed decompile corpus
(`~/Games/rebexe-decompile/decompiled-typed/FUN_<addr>.c`), the class
tables in `ghidra/generated/` (`class-slots.tsv`, `notifier-slots.tsv`),
and one disassembly (`objdump` of `REBEXE.EXE` at `0x41d100`). Builds on
`timer-scheduler.md` and `ai-mission-planning.md` ("Cadence", "The
fourteen modules"). "hyp" marks an inference no decompile states.

Tracker item: "AI na powiadomieniach i timerach". Short answer: the AI
is **polled once a sub-tick**, not woken by notifications or timers. No
timer `0x380..0x396` targets the AI. Notifications reach the AI player
only to mark its galaxy records dirty, and order replies are filed for
the planners. The port already has the cadence: `passes_per_day`, the
10-order budget, priority and round robin. It differs in **where** the
passes sit in the day and **when** they run in real time.

## The frame loop (`FUN_0041dff0`)

One call per frame runs the phase `this+8`. A phase that returns "not
done" runs again on the next frame. So the game clock waits for the
phase to finish. Phase 0 lets the loop go on only once 0.25 s have
passed and the clock is not held (`FUN_0041e390`; `FUN_0051de80` reads
bit `0x10000000` of `+0xc0`'s `+0x74`, which `victory.rs` reads as the
game-over hold). A full pass of phases 0 to 13 is **one sub-tick**. The
player manager is `DAT_006b14e4` (`thunk_FUN_00435a40`). Its `+0x10`
holds the side 1 (Alliance) player and `+0x14` the side 2 player
(`FUN_004365d0`).

| Phase | Call | Work |
|------:|------|------|
| 0 | `FUN_0041e390` -> `FUN_0051dea0` | sub-tick `+0xc` += 1; on the sub-tick after the in-day count reaches the rate, day `+0x10` += 1 |
| 1 | `FUN_0041e410` -> `FUN_0041d070` -> `FUN_00436550` | each player's slot `+0x30` (sub-tick): lockstep token (hyp) |
| 2 | `FUN_0041e300` | wait while the manager's `+0x10` equals the sub-tick (lockstep, hyp) |
| 3 | `FUN_0041cf40` -> `FUN_00436250` | each player's slot `+0x2c` (`Class_0065b070_vt2c`, `FUN_0048a960`): feeds the player's stored orders to the scheduler (`FUN_0041db80` -> `FUN_004fd220`) |
| 4 | `FUN_0041e430` -> `FUN_0040f340` | returns 1 |
| 5 | `FUN_0041e460` -> `FUN_0051df30` | serve the scheduler: due timers in queues `+0xb0`, `+0xa8`, `+0xac`, then queued events and orders (`+0xa4`, `FUN_0051dd20`), one item a frame until none is left |
| 6 | `FUN_0041e490` -> `FUN_0051e050` | flush the message queue `+0xb4` (`FUN_0051eaf0` -> `FUN_004fd300` -> hook `0x41d100` -> `FUN_004365d0`): each message goes to **its side's** player, slot `+0x50` |
| 7 | `FUN_0041d0a0` -> `FUN_00436580` | each player's slot `+0x34` (`FUN_0048a9f0`: `FUN_004fcdd0`, returns 1) |
| 8 | `FUN_0041cf20` -> `FUN_004361a0` | **the AI tick**: player `+0x10`'s slot `+0x24` until it returns 1, then player `+0x14`'s |
| 9..11 | `FUN_0041d0e0`, `FUN_0041d370`, `FUN_0041d390` | the local (human) player's UI queues and the session check |
| 12 | `FUN_0041e4d0` -> `FUN_0051e220` | sync snapshot every `0x28` sub-ticks (hyp) |
| 13 | `FUN_0041e500` | each player's slot `+0x38`; **when the scheduler's day-end flag `+0x24` is set**, each player's slot `+0x54` (`FUN_0041d050` -> `FUN_00436530`) |

`FUN_005136d0` (setup) and `FUN_0051ddf0` (frame message 9) also drain the
scheduler. Neither runs the AI.

### Where the day falls

`FUN_0051dea0` sets `+0x24` on the sub-tick where the in-day count
`+0x20` reaches the rate `+0x1c`. That rate is the larger of the sides'
`+0xc4`, so in single player it is the player's speed. The next
sub-tick raises the day and clears the count. A day therefore has rate+1
sub-ticks: 601, 61, 13 or 5.

Timers fire when the day reaches their due day: `+0x40` = day + delay
(`FUN_0051d8a0`). Reading the queue's slot `+0x1c` as "head `+0x40` <=
day" is still an inference (`timer-scheduler.md`). So every timer due on
day D fires in phase 5 of D's **first** sub-tick, before that
sub-tick's AI tick, along with every event and notification it raises.
Later sub-ticks of D find no due timer and serve only the events and
orders queued since.

## The AI player's tick (slot `+0x24`, `FUN_00485b40`)

The AI player is vtable `0x0065abf0` (`FUN_00485780`); its setup is
slot `+0x10`, `FUN_00485a50`. Its tick is the state machine `+0x50`. A
call that returns 0 is called again on the next frame, still in phase 8,
so game time stands still until the tick ends.

- **State 4: refresh the records** (`FUN_00418500`, one changed record a
  call). Its own state `+0x180` goes 4, 2, 1, 5:
  - 4: people, family `0x30..0x3f`, `FUN_00402230`;
  - 2: fleets, `FUN_00433620`;
  - 1: sectors with their systems, `FUN_004321e0`;
  - 5: when `+4` bit 31 is set, the controller records `+0xc0` and
    `+0x104`, then the side tallies by the bits that changed: bit 29
    `FUN_0041b7e0`, bit 30 `FUN_0041ba80`, bit 28 `FUN_0041bc10`.

  Each record refreshes **only when its dirty flag is set**: person
  `+0x30` bit 31, fleet `+0x1c`, sector `+0x20`, system `+0x24`
  (`ai-galaxy-records.md`). Then the tick moves to state 5.
- **State 5: ready the modules.** The cursor `+0x80` walks the module
  list and calls each module's slot `+0x10`. For 13 modules that is
  `FUN_004cc990`: a module not in state 1 or 2 becomes ready
  (`+0x1c` = 1, `+0x20` = 0). Module `0xe` uses `FUN_004d2970`, which
  also needs controller `+0x264` > 0, a ship to rename. The walk is
  skipped while `+0x418` is set; that field is only loaded and saved
  (`FUN_004858c0`, `FUN_00485990`), so hyp: 0. Then the tick moves to
  state 3.
- **State 3: steps.**
  - With the day's count `+0x420` at or past the budget `+0x428` (10),
    the tick ends at once.
  - Otherwise `FUN_004be980` picks a module: the highest priority
    `+0x34`, else the round robin.
  - Each call steps the current module once. A returned order is
    checked by its slot `+0x1c`:
    - accepted: it is submitted (`FUN_0048aa90`) and counted unless it
      is a Rename `0x203`;
    - refused: it goes to the module's slot `+0x18`.
  - A module that stays incomplete is stepped again next frame, still
    inside this tick, so one tick can issue several orders.
  - A complete module (`+0x1c` = 0, cursor cleared) ends the tick when
    its steps returned an order or when `+0x88` bit 31 is set
    (no writer found; hyp 0). Otherwise the next module is picked.
  - When no module is ready, the tick ends and returns 1.
- **The day-end slot `+0x54` (`FUN_00485de0`)** runs in phase 13 of the
  last sub-tick of each day. It clears the count `+0x420` once
  `+0x41c` reaches `+0x424`. `+0x424` is only loaded and saved, so the
  count clears every day.

The cursor `+0x80` is both the current module and state 5's list
cursor, and it is saved (`FUN_00485990`). A module still current when
the budget runs out is where the next tick's readying walk starts. The
modules before it in the list keep their earlier ready flag.

### Submitting an order

`FUN_0048aa90` gives the order a request key (the player's slot `+0x18`)
when it has none (`FUN_005f5600`). It then calls `FUN_0041cee0` ->
`FUN_004360f0`. The order goes to the other player's slot `+0x20` (for
an AI player, `FUN_0051ebb0`, which returns 1), then to its own
player's slot `+0x1c` (`FUN_0048a920`), which stores it in the order
table (`FUN_00536fe0`). Phase 3 of a later sub-tick passes stored orders
to the scheduler, and phase 5 runs them (hyp: the next sub-tick; the
lockstep in phases 1 to 3 is not traced). Both sides' orders from
sub-tick k therefore run in sub-tick k+1. Side 2's tick k does not see
side 1's order from tick k.

## What notifications do to the AI

`FUN_005400f0` builds a message from the UI factory table
(`FUN_0041e580`, `FUN_0051f770`). It stores the side bits of the object
the message is about at `+0x18` (`+0x24 >> 4 & 3`). Phase 6 hands the
message to that side's player (`FUN_004365d0`). For the AI player, slot
`+0x50` (`FUN_00485d10`):

1. calls the message's slot `+0x18` with the player. Each message class
   calls **one** player slot (visitor);
2. calls the message's slot `+0x1c(0)` when the message carries a
   request key `+0xc`;
3. calls `FUN_004bef90`, which deletes a message with no key. A keyed
   message, an order reply (kinds 2 to 4, `FUN_0051e6a0`..`FUN_0051e840`),
   is filed under its request (`FUN_004e4f20`). A reply of kind 2 sets
   the request's `+0x28`. hyp: the planner that issued the order polls
   it.

The player slots only mark records (`FUN_00418a60`: system, capital ship,
sector, fleet, person or mission record by the key's family) or set
controller bits. **No slot readies or steps a module.**

The table below was derived with a throwaway script (not kept): factory,
then constructor, then class vtable, then slot `+0x18`, then the player
slot it calls, then the AI player's function. Sender names
come from the `FUN_0053fcf0` / `FUN_0053fdd0` call sites.

| Messages | AI player slot | Effect |
|----------|----------------|--------|
| `0x120` SideMaintRequired | `+0x68` `FUN_00418990` | store the requirement at `+0x188`; tallies dirty (`+4` bit 31) |
| `0x128` SideVictoryConditions | `+0x74` `FUN_004189b0` | the game-type counts `+0x190`/`+0x194`; `+4` bit 1 |
| `0x125`/`0x126`/`0x127` Shipyard / TrainingFacil / ConstructionYard RdOrder | `+0xf4`/`+0xf0`/`+0xec` (`FUN_004198a0`, `FUN_00419870`, `FUN_004197b0`) | the research-done bits (port: `Planner::research_done`) |
| `0x140..0x144`, `0x149..0x14d`, `0x14e..0x155` system notifications (Loyalty, Energy, Raw material, Populated, Explored, Uprising, Battle, Blockade, Bombard, Assault, ControlKindUprising, incidents) | `+0x78`..`+0xc4` | `FUN_00418a60`: mark the system record |
| `0x100` ControlKind | `+0x150` | controller `+4` bit 1 for a system, then mark |
| `0x156` TroopRegRequired (`+0xc8`), `0x101` Usable, `0x104` Enroute, `0x106` Damaged, `0x1a0` SquadSize, `0x1c0..0x1c6` capital-ship allocations, `0x160` ManuMgr remaining, `0x1e0`, `0x1e1` Force, `0x200` Role | `+0x160`, `+0x158`, `+0x164`, `+0x124`, `+0x108..+0x120`, `+0xcc`, `+0x13c`, `+0x138`, `+0x134` | mark the object's record |
| `0x102` Created, `0x103` Completed, `0x107` Deployed, `0x380` | `+0x148`, `+0x168`, `+0x144` | mark |
| `0x302` Destroyed | `+0x16c` | mark, then `FUN_00418db0` (drop the record) |
| `0x305` | `+0x170` `FUN_00419110` | drop the record of a sabotaged object (hyp from the name) |
| `0x300` Existing | `+0x14c` `FUN_00419260` | mark the system, or the capital ship with its arguments |
| `0x180`/`0x182`/`0x183` Fleet Battle / Bombard / Assault | `+0xfc`/`+0x104`/`+0xf8` | mark the fleet when the flag argument is set |
| `0x1e2` CommandKind | `+0x140` `FUN_00419630` | mark the person and the place it is at |
| `0x1f0`, `0x1f1` (mission messages, `FUN_00525040`, `FUN_00525070`) | `+0x128` `FUN_004195f0` -> `FUN_0042f7f0`; `+0x12c` `FUN_00419610` -> `FUN_0042f810` | the mission record (research planners' Mission Report) |
| `0x370` Espionage ExtraSystem | `+0x130` | mark, then `FUN_00419740` (the system seen) |
| `0x145` Smuggling, `0x146` ProductionModifier, `0x161`/`0x162`/`0x163` ManuMgr counts, `0x105` EnrouteActive, `0x181` FleetBlockade, `0x121..0x124`, `0x320`, `0x321`, `0x400`, `0x520` | no-op slots (`FUN_00403ef0`, `FUN_005ae1e0`, `FUN_0048b3d0`) | nothing |
| `0x147`, `0x148` (TroopReg withdraw / surplus), `0x1e3` Injury, `0x1e4`, `0x1e5`, `0x210..0x231`, `0x303`, `0x304`, `0x306` | message slot `+0x18` is empty | nothing |

So a record changes only when one of its marking notifications arrives.
A change with no marking notification of its own (the no-op rows) reaches
the AI only when something else marks the same record.

## Modules and how often they run

All fourteen modules are readied on every tick. Module `0xe` also needs a
ship to rename. Inside a tick, modules run by priority and then round
robin until one gives an order. The day allows 10 orders, but a Rename
does not count. A module's own triggers are record bits, such as an idle
construction yard or a sector candidate, which the refresh rebuilds from
the dirty records. Modules have no timers and no per-module wake-up
events.

| Speed | Rate | AI ticks a day | Real seconds a day |
|-------|-----:|---------------:|-------------------:|
| Fast | 4 | 5 | 1.25 |
| Medium | 12 | 13 | 3.25 |
| Slow | 60 | 61 | 15.25 |
| Very Slow | 600 | 601 | 150.25 |

At Fast the order budget never binds, because each tick takes at most one
completing module with an order. From Medium up, it can.

## The port, compared

Updated 2026-11-20 (stages 3 and 4). The list below "Differences a player
can notice" describes the port **before** these stages; see "Staged port
plan" for what is done.

- `ai.rs` (`AISystem::pass`, `passes_per_day`, `ORDERS_PER_DAY`,
  `next_module`, `MODULE_IDS`, `AIState::begin_day`) matches the tick:
  refresh, ready, then steps, one order a completing step, and 10 a day.
- `rebellion-core/src/ai_schedule.rs` is the one pass driver both day
  paths use. `run_pass` asks each side for its tick against the same
  world, the Alliance's first (`tick_order`: `FUN_004361a0`, `+0x10`
  then `+0x14`), then applies both sides' orders, the Alliance's first,
  then settles once if any applied. So neither side's tick sees the
  other's orders of the same pass. `AiDay` holds the day's place: its
  passes (the speed's sub-ticks, fixed when the day begins), the passes
  run, the bombardment stream's state, and the end of the day owed (the
  loyalty watch and the days it closes). It is saved (v132).
- Each host supplies `AiPassHost`: `HeadlessPasses` in
  `rebellion-data/src/simulation.rs` (the integrator) and `AppPasses` in
  `rebellion-app/src/main.rs` (the messages). A pass that applied orders
  settles, in both: its bombardment and assault reports and the planets a
  Death Star destroyed, then the blockade (`BlockadeSystem::detect`) and
  the victory word. A win holds the day's remaining passes.
- The day splits in three in both paths: the start (every system up to
  the day's timers, then the regiments that set out since the last day
  running a blockade, then `AiDay::begin`), the passes, and the end once
  the last pass ran (the regiments the day's orders sent through a
  blockade, the player's agent in the app, loyalty, the products, the
  views and victory). `rebellion-data` names them
  `begin_simulation_day`, `run_ai_passes` and `close_simulation_day`;
  `run_simulation_tick` runs all three, so the playtest and replays keep
  the burst. The app runs one pass as each sub-tick begins
  (`AiDay::due(GameClock::day_part)`): the first with the day's start,
  the rest as real time passes, and the day's end after the last; a frame
  that reaches the next day first runs what is left of the day before. A
  held or paused clock holds the remaining passes; a queued play-test
  day runs them at once.
- The burst and the spread give the same campaign when nothing comes
  between sub-ticks (`replay.rs`,
  `spreading_a_day_s_ai_passes_over_its_sub_ticks_matches_the_burst`,
  seed 42, dual AI, Medium, 30 days).
- The port rebuilds every record on every pass
  (`ai_records::Galaxy::build`). Stage 6 made `pass` refresh even when
  the day's budget is spent.

### Differences a player can notice (before stages 3 and 4)

1. **The AI acts at the day boundary, not through the day.** At Slow, the
   port issues a day's orders (fleet departures, builds, missions)
   together when the day turns. The original issues them one sub-tick
   (0.25 s) apart over 15 s, or over 150 s at Very Slow. An order the
   player gives during day D reaches the original AI at its next
   sub-tick, the same day. In the port it waits for day D+1's first
   pass. At Fast (1.25 s a day) this is barely visible.
2. **Day-start timers come after the AI in the port.** In the original,
   every timer due on day D runs before D's first AI tick, together with
   the events and notifications it raises. In the port the AI sees the
   following one day late:
   - an uprising, disaster or resource incident (`0x38d`, `0x38f`,
     `0x390`);
   - support moved by the revolt and blockade timers (`0x384`, `0x386`);
   - informant reports (`0x38e`);
   - a healed character (`0x388`);
   - Force discovery (`0x389`);
   - a repaired ship (`0x380` / repair). Module `0xb` may then send a
     ship to a yard that the same day repairs;
   - mine and refinery output and the stockpiles behind module 1's
     maintenance tallies (`0x394`);
   - an autoscrapped unit (`0x382`). The AI may give it an order on the
     day it disappears;
   - the yards' own research points (`0x381`). New classes become
     buildable a day later;
   - loyalty shifts.

   The blockade bit of a system record is read live
   (`system_is_blockaded`), so a new blockade is seen at once. Yards a
   blockade suspends (`suspended_systems`, from `BlockadeState`) are
   not.
3. **Order of the AI's own consequences.** The port's after-AI systems
   (blockade, uprising, loyalty, victory) see the AI's same-day orders
   on day D. In the original, effects driven by events, such as control,
   blockade and the victory word, follow at once (sub-tick k+1).
   Effects driven by timers come on a later day. Moving the AI later
   must keep the event-driven part after each pass.
4. **Dual AI: side order and visibility.** The original ticks the
   Alliance (player `+0x10`) and then the Empire in every sub-tick, and
   both sides' orders run in the next sub-tick. The port defaults to
   the Empire first and applies each side's orders before the other
   side's pass in the same sub-tick. This matters only for dual-AI
   playtests and replays.
5. **Record freshness.** The original's records lag where a change sends
   no marking notification (table above). The port's are always fresh.
   No case where the AI decides differently has been found; it needs a
   field-by-field audit.
6. **The app and the playtest order the after-AI systems differently.**
   This was F-012 (closed for the order 2026-11-20, see "Status"). For
   example, the app ran mines and refineries,
   research and autoscrap before uprising, healing and informants; the
   playtest runs them after. Either way, all of them come after the AI.

Small and invisible: when the budget is spent, the original still
refreshes and readies on every tick. The port skips both, and the only
state they keep is the `seen` dates of record `+0x44`.

## Staged port plan

Each stage is gated and committed on its own. A new or changed rule
carries its source (the `FUN_` addresses above).

Status 2026-11-20 (afternoon): the day has one implementation. The app
packs its states into `SimulationStates` and runs the same functions as
the headless runner: `begin_day` (`day_head` with `run_day_missions`,
then `run_day_timers`), `start_ai_day`, `run_ai_passes` and
`close_simulation_day`. Its messages, movies, sounds, advisor lines and
the tactical battle come from `DayPresentation` hooks, which see each
step's events before and after they are applied (`AppDayPresentation`).
The runners still differ in two ways the audit's F-012 acceptance names:
the app draws its rolls from its generator as asked (`RngRolls`), the
runner from a 1024-roll slice a day padded with 1.0 (`SliceRolls`); and
the app fights a battle with both sides' fleets as a tactical battle
(`take_battle`) while the galaxy shows.

Status 2026-11-20 (later): stages 3 and 4 are done
(`rebellion-core/src/ai_schedule.rs`, save v132). Stage 5 is not done: no
case where a stale record changes a decision a player can see has been
found, and the field audit it needs is still open.

Status earlier on 2026-11-20: stages 1, 2 and 6 are done in both day paths
(`rebellion-data/src/simulation.rs` and the app), without the F-012
unification. Stage 1 also moves the blockade's dawn look (arrivals and
timer `0x386`) ahead of the passes; the regiments running a blockade
stay after them. Stage 2 recomputes the blockade (`BlockadeSystem::detect`,
the losses of the pass first) and the victory word after each pass that
applied orders, and a win stops the day's remaining passes
(`settle_after_pass`, `settle_after_ai_pass`).

1. **Day-start timers before the AI (order only).** In
   `run_simulation_tick` and the app loop, move the timer systems ahead
   of step 7:
   - uprising, disaster and resource incidents;
   - support timers;
   - informants, healing, Force discovery;
   - Dagobah and Jabba are already before;
   - repair, mines and refineries, autoscrap, the research tally.

   Keep the derived systems after the passes: blockade, loyalty, object
   views, victory. They must also run, as today, on the AI's orders.
   - Risk: the roll cursor (`take_rolls`) order changes, so every replay
     golden and seeded playtest changes. Regenerate them, naming
     "recovered original day order (ai-scheduling.md)". No save change.
   - Tests:
     - `an_uprising_that_starts_at_dawn_is_seen_by_the_first_ai_pass`
     - `a_unit_autoscrapped_at_dawn_gets_no_ai_order_that_day`
     - `research_points_from_the_yard_tally_reach_the_ai_the_same_day`
   - Do F-012 (one shared day function for the app and the playtest)
     first or together, or the two orders drift further.
2. **Event-driven consequences after each pass.** Split blockade and the
   victory word into "recompute from the world" functions. Call them
   after every pass that applied orders, not once after the burst, as
   the original's handlers fire at sub-tick k+1. Risk: cost at 601
   passes. Gate the calls on "orders applied this pass" (most passes
   apply none). No save change. Tests: a bombard or assault order's
   control change is seen by the next pass of the same day.
3. **One pass per sub-tick in real time (app).** Have the clock emit
   sub-tick events. The app runs the day systems on the rollover
   sub-tick, then one pass per side per sub-tick, applying player orders
   between sub-ticks. `rebellion-playtest` keeps the burst, which is
   identical when there is no outside input. Prove that with a test
   comparing burst and per-sub-tick driving on a seed.
   - Save: persist the in-day pass index; the AI state already holds
     `orders_today`. This bumps `SAVE_VERSION` (approved; say so in the
     commit).
   - Replays: player orders need a sub-tick stamp.
   - Tests:
     - `a_day_runs_rate_plus_one_passes_one_per_sub_tick`
     - `a_player_order_given_mid_day_is_seen_by_the_next_ai_pass`
     - `pausing_mid_day_holds_the_remaining_passes`
     - `loading_a_mid_day_save_resumes_at_its_pass`
   - Done 2026-11-20. Replays need no sub-tick stamp: format v1 records
     commands between whole days, and its runner keeps the burst, so a
     command still lands between a day's end and the next day's start.
     The app records no replay.
   - port: the day's start and its end stay where the burst had them, so
     the end of the day (loyalty, views, victory check) comes after the
     day's last pass rather than as each event fires. The regiments that
     set out since the last day roll before the first pass (they rolled
     after the passes before); those the day's orders send roll at its
     end.
4. **Order latency and side order (dual AI).** Run the Alliance's pass
   first (`FUN_004361a0`, `FUN_004365d0`). Hold both sides' accepted
   orders from pass k and apply them before pass k+1. A human's order
   given at sub-tick k likewise applies at k+1. Risk: dual-AI outcomes
   and goldens change. No save change unless held orders can span a
   save, in which case save them (hyp: they cannot, since a pass and its
   application happen in one call).
   - Done 2026-11-20 (`ai_schedule::run_pass`). Held orders never span a
     save: a pass thinks and applies in one call.
5. **Record dirtiness (only if the audit in item 5 above finds a visible
   case).** Keep per-object records in `AIState` and refresh one only
   when a port event equivalent to its marking notification fired.
   - Risk: high. Records are large, they would have to be saved (save
     bump), and hidden staleness bugs are hard to test.
   - Recommendation: do the field-by-field audit first and port only the
     fields whose staleness changes a decision.
6. **Refresh while the budget is spent.** Let `pass` run its refresh
   side effects (the `seen` dates, the holds' `retain`) even when
   `orders_today >= ORDERS_PER_DAY`. Trivial; test
   `a_spent_budget_still_dates_a_seen_blockade`.

Other agents are working on module `0xc` and the plan slots. Nothing
here touches them; stage 4 changes only the driver.

## Open

- The lockstep phases 1 to 3: the exact sub-tick at which an AI order
  runs. The trace says next sub-tick; this is hyp.
- The writer of `+0x88` bit 31 (end the tick after a module completes
  without an order) and of `+0x418`.
- Who polls the order replies `FUN_004e4f20` files (`+0x28`), and
  whether a refused execution (reply kind 2) changes a module's state.
- The queue slot `+0x1c` due test (`timer-scheduler.md`) and timers with
  minimum delay 0, which would fire in the sub-tick that arms them.
