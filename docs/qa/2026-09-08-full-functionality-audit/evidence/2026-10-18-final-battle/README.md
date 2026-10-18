# The Final Battle (F-038)

Date: 2026-10-18.

The port's Final Battle was a chain of its own story events: `0x393`
Vader Dispatched, `0x394`, `0x395`, `0x396` Final Battle Imminent, and
`0x220` The Final Battle. They fired on Dagobah's end, a tick threshold
and the three characters meeting, with no outcome. This stage replaces
them with the original's FinalBattleReady path, read in
`ghidra/notes/final-battle.md` (plan 1).

## The original

- Luke's own FinalBattleReady (`+0xb0` bit 2, `FUN_0056f7a0`) holds when
  he knows his heritage and has Force of at least GNPRTB 3078 (60).
- FinalBattleReady (`FUN_00544da0`) holds while FinalBattle is clear and
  all of the following are true:
  - Luke is captured, on no mission, not autorouting, and not in Jabba's
    Palace, and his own flag is set;
  - Vader and the Emperor are free.
- DarthToLukeFinalBattle (`FUN_00544a20`): while Vader stands elsewhere,
  its hook orders him to Luke's location by a Move order (`FUN_0054ac70`,
  `FUN_00547340`, `0x41000001`).
- DarthToEmperorFinalBattle (`FUN_00544be0`): with Vader beside Luke and
  the Emperor elsewhere, the hook moves Vader with the captive Luke to the
  Emperor (`FUN_0054adf0`). Both must be the Empire's and on no mission.
- FinalBattle (`FUN_00544fc0`) is set when the three meet, and its hook
  fights the battle (`FUN_0054b040`):
  - both sides observe the system;
  - with Luke's Force below GNPRTB 6151 (100) the Empire wins: Luke takes
    rand(0..=6153 (199)) + 6152 (1) injury, stays captive and loses
    `CanEscape`;
  - otherwise Luke goes free, and Vader and the Emperor are captured by
    his side and lose `CanEscape`;
  - a winner standing where another side holds takes the three to his
    autoroute destination by a Move order.
- The encounter class posts The Final Battle (`FUN_0048e3c0`): `0x71e0`,
  then `0x71e1` (Luke won) or `0x71e2` (Luke captured), on background
  `0x427`, or `0x428` after Dagobah.

## Port

- `final_battle.rs`:
  - `FinalBattleState` (save v76);
  - `FinalBattleSystem::advance` (the day's checks, in `FUN_00549910`'s
    order);
  - `perform` (the hooks: Move orders and the battle);
  - `luke_ready`.
- Simulation step 5c and the app run it after Dagobah. The app posts
  `story_messages::final_battle_message`, and the telemetry records
  `story_final_battle` per step.
- `Character::escape_barred` (save v76): `CanEscape` cleared while held.
  The escape timers skip such prisoners, and a new capture clears the
  flag (`FUN_004f1a80`).
- A prisoner's side in a mission's member checks is its captor's
  (`FUN_004eec60`). Both `dispatch_guarded` and `members_allowed` read it.
  Before this, the Empire's Move order refused the captive Luke
  (`MembersNotAllowed`).
- `story_events.rs` no longer defines `0x393`..`0x396` or `0x220`.
- port:
  - the checks run daily;
  - Luke's own flag is derived from its two inputs;
  - "free" has no usable or adrift bit;
  - the Move orders target the location's system;
  - Luke's freeing sets no Autorouting, because the hook's Move order
    takes him home.
- Not ported: DarthPickup and the Pickup mission (`0x73`), which need
  Jabba's palace.

## Tests

- `final_battle.rs`, 12 tests:
  - Luke's own flag;
  - FinalBattleReady's conditions;
  - Jabba's Palace;
  - Vader sent to Luke by a Move order;
  - Vader taking the captive Luke to the Emperor (in the captured list);
  - that hook's side and mission conditions;
  - one battle only;
  - the defeat's injury (1 to 200), captivity and barred escape;
  - the victory's captures and the Move order home;
  - a barred prisoner gets no escape timer, and a new capture clears the
    bar;
  - the roll budget.
- `story_messages.rs`: `the_final_battle_message_tells_luke_s_fate`.
- `integrator.rs`: `a_final_battle_step_is_recorded_with_its_outcome`;
  `no_story_event_reveals_luke_s_heritage` replaces the test that asserted
  `0x396` fired.
- `story_events.rs`: `the_invented_final_battle_chain_is_gone` replaces
  `final_battle_chain_requires_dagobah_completion`. The Emperor test drops
  its Final Battle assertions.
- Shown failing without the change:
  - with the prisoner side and the escape bar reverted in `missions.rs`, 3
    tests fail;
  - with `members_allowed`'s side reverted, 2 fail (the test world then has
    the shipped Move record).

## Campaigns (`playtest-final-battle.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1, 2, 3, 7, 13, 42 and 47,
1500 days.

- Seed 13:
  - Luke learns his heritage on day 1095;
  - on day 1140 Vader takes the captive Luke to the Emperor (Qat
    Chrystac);
  - the battle is on day 1151: the Empire wins, and Luke takes 75 injury.
- Seed 47:
  - heritage on day 683;
  - the trip to the Emperor at Denab on day 762;
  - the battle on day 854: the Empire wins, and Luke takes 64.
- Before the side fix, the trip was refused every day (`MembersNotAllowed`
  in a debug run).
- Luke's Force stayed below 100 in both, so neither campaign shows his
  victory.

## Browser

- The story-message fixture (Alliance, code 312) lists The Final Battle,
  checked with `agent-browser`:
  - text: "Luke has been brought before the Emperor by Vader. Luke
    confronted and overcame the Emperor. Both Emperor Palpatine and Darth
    Vader were lost.";
  - background `0x427` (1063), the throne room.
- Screenshot not committed (original artwork):
  `ac5a7b91c19ac96767366c359befe935101703cfc4640029a0657530c18e689c
  target/acceptance/final-battle/final-battle.png`.
- The Fleet window gate passes all 48 cases (interface WASM
  `60e2fd48d360e22f1e078707c2c35f5749eeb45d3da97982d25a06f899efe5ba`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.7 ms | 24.4 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 3.3 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- Performance screenshots, not committed: `1-veryslow.png`
  `f5c0796cf13ff14b29beb7ae520b651968d6681d6e4b0de51cf00f03a38b21b2`,
  `2-fast.png`
  `e34c00bcfcb8ac965eececf1cc1636197ae0f209df3d0fb4354ba74164945c40`.
- Native GUI acceptance was not run for this change.

## Gates

- `cargo test --workspace`: 2379 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v76, the Final Battle
  state and its daily roll):
  - initial `v1:ad8518eb172a430f` -> `v1:978a2af596955f63`;
  - final `v1:e811b12322f7e20e` -> `v1:8c0677e665d1d127`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `d950ae70ecccb4247e87a264e46e1703ad7e377f0a3e507f01fd3ec5a4a0dbaf`.
- `cargo mutants --in-diff` (`final_battle.rs`, `missions.rs`):
  - first run: 80 mutants, 3 missed. They are closed by a test of the
    Emperor hook's conditions and of the roll budget;
  - second run: 80 mutants, 76 caught, 4 unviable.
- `story_messages.rs`: 2 mutants, 1 caught, 1 unviable.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt on the changed lines: clean, apart from
  the `implicit_hasher` the story builders share.

Hashes of the committed files are in `SHA256SUMS`.
