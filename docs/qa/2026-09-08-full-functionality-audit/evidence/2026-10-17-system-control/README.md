# Who holds a system: `FUN_0050b610` (F-037, F-006)

Date: 2026-10-17.

After the automatic landing was retired (`1215eca6`), neutral planets
flipped back and forth: step 3b gave a planet to the side whose regiments
stood on it, and the next day the economy made it neutral again. Reading
the economy's rule (`FUN_0050b610`) showed that the port had misread it.
The reading is in `ghidra/notes/system-control.md`.

## The original

`FUN_0050b610` fills three fields of the system's word `+0x84`:
- the regiments' side: one side's regiments alone give that side, both
  sides' keep the previous side, and no regiments give none;
- the support side: an unpopulated system has none. Otherwise
  `FUN_0055a080` compares the Alliance's support `+0x58` with
  `DAT_006bb4c0`, which is GNPRTB 7760 (60). At least 60 gives the
  Alliance, 100 less the support at least 60 gives the Empire, and
  anything between gives no side;
- the holder: the regiments' side when there is one, else the support
  side.

The original runs it in the state rebuild (`FUN_00508250`) and also in the
system's handlers when its contents change (`FUN_005109f0`,
`FUN_00510e30`).

The port had differed in three ways:
- it compared the system's energy with GNPRTB 7760;
- with both sides' regiments it returned `Contested`;
- it made every unpopulated system neutral, even with regiments on it.

## Port

- `economy::resolve_system_control(sys, alliance_troops, empire_troops,
  gnprtb, difficulty)` follows the rule above. `economy::system_holder`
  counts the regiments as `FUN_00504cc0` does, at any strength.
- The economy's step 5 uses it. The simulation's step 3b and the app's
  `tactical_flow::occupy` use it too, for systems with regiments on the
  planet, in place of the handlers' call. All three now agree.
- hyp: the port keeps no regiment bits, so with both sides' regiments the
  previous side is the side holding the system, if any.
- port: a side's uprising stays while that side holds the system, and a
  neutral holder (`Controlled(Neutral)`) keeps its kind.
- The interface fixture gave the target planet 60 energy for the misread
  rule. It now gives both planets the player's support (90%).

Save format: unchanged at v59.

## Tests that changed

- `economy.rs`: four tests asserted the misread rule. They are replaced:
  - `side_resolution_alliance_troops_only`;
  - `side_resolution_both_troops_contested`, which expected `Contested`;
  - `side_resolution_no_troops_energy_above_threshold_preserves` and
    `side_resolution_no_troops_low_energy_becomes_uncontrolled`, the
    energy test.

  The new tests are:
  - `regiments_of_one_side_alone_hold_the_system_and_both_keep_the_holder`;
  - `without_regiments_the_side_with_60_support_holds_the_system`;
  - `the_holder_counts_any_regiment_on_the_planet`;
  - `the_economy_s_run_gives_a_system_to_the_side_of_its_regiments`.
- `simulation.rs`:
  - new: `a_regiment_arriving_takes_its_planet_the_same_day`;
  - three fixtures built on `GameWorld::default()` had no GNPRTB, so the
    threshold was 0 and every system without regiments became the
    Alliance's. They now carry GNPRTB 7760 (60), and each system gets its
    holder's support. Their assertions are unchanged.
- `tactical_flow.rs`: a regiment at strength 0 used to be skipped by the
  occupation. `FUN_00504cc0` counts it, so with both sides' regiments the
  Alliance keeps the system. That case is now separate.

## Campaigns (`playtest-control.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, compared with
`c4b8cfa4`:

| Seed | Control changes (old → new) | Systems changing 3+ times (old → new) |
|---|---|---|
| 1 | 10 → 18 | 1 → 1 |
| 2 | 11 → 12 | 1 → 0 |
| 3 | 19 → 17 | 2 → 0 |
| 7 | 13 → 16 | 1 → 0 |
| 13 | 11 → 13 | 0 → 1 |
| 42 | 6 → 13 | 0 → 0 |
| 47 | 8 → 10 | 0 → 0 |

- The same-day clash is gone.
- The two systems still changing 3 or more times are Sullust in seed 1
  and Umgul in seed 13. Their changes are weeks apart: the Empire's
  regiments take the system, and once they have gone the Alliance's
  support takes it back.
- More systems change hands, because a system without regiments now
  follows its support and no longer its energy.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`458e95d5ecced246670cece204768b9759f763ab9621ede3a42c71e036558188`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast. Each
frame's work time was measured:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3600 | 0.6 ms | 0.8 ms | 22.9 ms |
| Fast | 60 s | 3600 | 0.6 ms | 1.6 ms | 1.8 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2245 passed, 0 failed.
- Neutralized: 10 changes, all failing their tests. They are:
  - in the rule: both sides' regiments, the unpopulated system, the two
    support thresholds, the regiments first, the uprising, the neutral
    holder and the strength;
  - the uses: step 3b and the app's `occupy`.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for the corrected rule. The initial
  fingerprint `v1:e47089c0adbf9615` is unchanged; from the checkpoint
  after 3 commands the fingerprints change, and the final goes
  `v1:9234ac6ef0f26a8a` -> `v1:ef80035d2be8099a`. Two recordings are
  identical, and only the fingerprints changed in the manifest. The replay
  equivalence check passes with 0 browser errors. Release WASM SHA-256
  `bc7377d85d99904fca494dd46d27de80ba095da4223b8dc8897602d3f367ee09`.
- `cargo mutants --in-diff`: 31 mutants, 20 caught, 2 unviable, 9 missed
  in the first run.
  - Six missed in the economy's step 5 regiment counts. The new economy
    test now catches them.
  - Three are in the interface fixture's support line. The Fleet window
    gate passes even with the support flipped (tried), because no case
    checks the target's holder now that nothing lands by itself.
- `provenance-scan check`: 99 uncited items, one fewer.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
