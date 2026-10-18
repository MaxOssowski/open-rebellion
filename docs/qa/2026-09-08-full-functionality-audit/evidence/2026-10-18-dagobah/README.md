# Luke's Dagobah mission (F-038, F-040)

Date: 2026-10-18.

The port's Dagobah was a chain of its own story events: "Dagobah Calls"
at day 30, Yoda, Luke's mandatory Jedi training from day 50, and the end
once his invented Force tier reached Experienced. This stage replaces it
with the original's mission manager trigger and Dagobah mission. The
reading is in `ghidra/notes/dagobah.md`.

## The original

- Timer `0x392` (`FUN_00549d60`): GNPRTB 6146 (300) + rand(0..=6147 (100))
  days after the start. It sets `LukeDagobahRequired` and is then disarmed.
- The Dagobah check (`FUN_00544030`): with the requirement set and Han not
  held at Jabba's palace, once Luke has a location and is free (not
  captured, not autorouting, not adrift, not on a mission), `LukeDagobah`
  pulses. Its hook (`FUN_0054a860`) creates mission `0x71000043` for Luke,
  with the Dagobah abode `0x98000441` as the target.
- The trip to the abode takes 1 day (`FUN_00555b30`).
- Phase 8 records the first training day (`FUN_005751d0`) and waits 100
  days.
- Phase 10 succeeds: chance 100 (`FUN_00575060`), result 3 (`FUN_00572470`).
- Phase `0xb`, after phase 8:
  - Luke's Force rises by GNPRTB 6174 (60) percent on a success, else by
    (days / GNPRTB 6175 (2)) percent, into `ForceExperience`;
  - `DagobahCompleted` is set;
  - the requirement clears (`FUN_00548840`).
- The members stay in the abode. The manager's `FUN_00546370` runs each
  one's slot `+0xb0` (`FUN_004ef260`), and Luke, standing in a side-3
  container, autoroutes home (`FUN_00543040`), 1 day out of the abode.
- Messages, both shown to the Alliance only:
  - `0x221` Luke Goes to Dagobah (`FUN_0048d190`, TEXTSTRA
    `0x71b8`/`0x71b9`, picture `0x421`);
  - `0x210` Luke Leaves Dagobah (`FUN_0048d0f0`, `0x71ba`/`0x71bb`,
    background `0x412` with Luke's portrait).

## Port

- `dagobah.rs`:
  - `DagobahState` (save v73);
  - `DagobahSystem::advance`, which arms and fires the timer and runs the
    check;
  - `dispatch` and `training_end`.
- `missions.rs`:
  - `MissionKind::Dagobah`;
  - `ActiveMission::target_abode`, `first_training_day` and `succeeded`;
  - `MemberTransit::abode`, for the 1-day trip;
  - `MissionEffect::MemberEnteredAbode` and `DagobahCompleted`;
  - `enter_abode`;
  - `leave_abode`, the autorouting out, sharing `autoroute_destination`
    with the escape;
  - `aim_at_abode` and `is_autorouting`.
- `Character::dagobah_completed` (save v73).
- The simulation runs the trigger after the missions (step 5b) and records
  `luke_dagobah` and a `force_experience` with cause `dagobah`. The app
  runs it too and posts both messages.
- `story_events.rs`:
  - the invented 0x390, 0x392, 0x221, 0x391 and 0x210 events are gone;
  - the events that waited for 0x210 ("Vader Dispatched", "Leia Discovers
    the Force") now wait for Luke's `DagobahCompleted` (new
    `EventCondition::CharacterDagobahCompleted`).
- port:
  - the check runs each day, where the original runs it from the
    manager's event handlers;
  - the port has no capture at Jabba's palace, so `HanCapturedAtPalace`
    never holds;
  - the mission keeps Luke's starting system in `target_system`, and the
    running rules skip the abode target, which can never be lost;
  - the autorouting origin is the system Luke left from;
  - `FUN_00520ac0` is read as the phase the mission leaves (hyp).

## Tests

- `missions.rs`:
  - `luke_trains_on_dagobah_for_100_days_and_autoroutes_home`: dispatch,
    the 1-day trips both ways, the first training day, 100 days, Force
    50 -> 80, `DagobahCompleted`, and Luke home again and free;
  - `only_a_member_of_another_side_than_the_abode_s_autoroutes_out`;
  - `only_dagobah_ended_after_phase_8_completes_dagobah`;
  - `a_killed_character_or_an_unknown_abode_takes_no_one_in`;
  - `an_unfinished_dagobah_changes_no_force` (and the days formula).
- `dagobah.rs`:
  - `timer_0x392_requires_dagobah_once_300_to_400_days_in`;
  - `luke_leaves_only_when_free_and_until_his_training_is_done`.
- `story_messages.rs`: `luke_s_dagobah_messages_go_to_the_alliance_only`.
- `integrator.rs`: `a_luke_dagobah_pulse_is_recorded`,
  `dagobah_s_end_is_recorded_with_luke_s_new_force`.
- `story_events.rs`:
  - `the_invented_dagobah_chain_is_gone` replaces
    `luke_dagobah_chain_fires_in_sequence`, which asserted the removed
    events.
  - `one_shot_story_events_do_not_refire` now uses the Force Awakening
    milestone, with the same assertions.
  - `final_battle_chain_requires_dagobah_completion` and
    `leia_force_discovery_fires_after_dagobah` set `DagobahCompleted` where
    they made Luke Experienced. They no longer assert the 0x210 event.
- Shown failing without the change: no abode trip, or no timer fire, fails
  the flow and timer tests.

## Campaigns (`playtest-dagobah.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1, 2, 3, 7, 13, 42 and 47,
against `bcf6b772`.
- Every seed differs from day 31 to 50, where the removed events fired.
- Over 1200 days, Luke leaves for Dagobah on seeds 2, 7, 13 and 47 (days
  302 to 513). He trains 100 days and ends at Force 80 (+30).
- On seeds 1, 3 and 42 an Empire assassination kills him first (days 322
  to 393).

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`bc0cd65e86679e9f32c329e653910b775f82f61830dfe1e89a787c5804dfeab7`).

Interface fixture scenario 55 (Alliance, `?fixture-code=312`) now also
posts the two Dagobah messages. Seen with `agent-browser`:

| Message | Picture |
|---|---|
| "Luke Goes to Dagobah", "Luke has been sent to Dagobah to be trained by Yoda." | `0x421` (1057), Luke and Yoda |
| "Luke Leaves Dagobah", "I have finished my training with Yoda." | `0x412` (1042) with Luke's portrait `0x1842` (6210) |

The screenshots show original artwork, so they are not committed
(`target/acceptance/dagobah-messages/`):

```
65a5ca821a21ee15117391ab32587fb53365341995a82fa9a735a138f537c8f6  alliance-list.png
2e769b63c7337fa044c241bb924b219c15f73e23ecec230aeec86c95ee412065  luke-goes-to-dagobah.png
d1ba8efbcbebba1ed809094a76c4e4c8e92ac17a23047719f08a31e1fbc4fd5f  luke-leaves-dagobah.png
```

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.6 ms | 0.8 ms | 25.9 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.1 ms |

- No frame took over 50 ms.
- The two performance screenshots show original artwork. They were
  removed from the repository on 2026-10-18 and kept in
  `target/acceptance/dagobah-perf/`:

```
f5c0796cf13ff14b29beb7ae520b651968d6681d6e4b0de51cf00f03a38b21b2  1-veryslow.png
e4dc89c703a9ae4ab2a9f4b5fc2e1fb69f44d7b1f1a65a51904db1f435d79757  2-fast.png
```

- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2327 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v73, the Dagobah trigger
  and mission, the removed story events):
  - initial `v1:c4aa6c7fc51080c1` -> `v1:7de82effa8207c75`;
  - final `v1:0e985b5a40e2bfa8` -> `v1:ab2d9eddde745c32`;
  - two recordings are identical, and only the fingerprints changed in the
    manifest;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `99aa9f466e04730b945675f965595f6e39ee7573345a51d1fd3ab3253ddc06bc`.
- `cargo mutants --in-diff`:
  - first run: 119 mutants, 18 missed. The tests above, on the exact trip
    days, the autorouting cases, other kinds and the killed Luke, close
    them.
  - second run: 119 mutants, 102 caught, 14 unviable, 3 missed:
    - the fleet roster in `enter_abode`, now covered by the last test (a
      run over `enter_abode` alone: 4 mutants, 4 caught);
    - `convert_character`, the DAT loader's old gap;
    - the legacy missions panel's label for the new kind.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021) on the changed lines: two
  `implicit_hasher` findings on the texts map, as the neighbouring story
  builders take, and nothing else.

Hashes of the committed files are in `SHA256SUMS`.
