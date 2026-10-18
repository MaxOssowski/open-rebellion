# Force detection and the encounter (F-038)

Date: 2026-10-18.

The port's mission detection had no Force detection. Luke's heritage and
his confrontation with Vader were story events of its own. This stage
ports both into the mission detection, as the original runs them. The
reading is in `ghidra/notes/encounter.md`.

## The original

- Force detection (`FUN_00589e40`) runs after the detection setup and
  before betrayal.
- The mission's characters fill two sets (`FUN_0058a5f0`), each counted by
  `FUN_005883b0`:
  - `+0x50`: the major characters of the team and decoys;
  - `+0x58`: the minor ones.
- The defenders' Force users fill two more (`FUN_0058a6c0` ->
  `FUN_0058a7e0`):
  - `+0x68` holds the majors and `+0x60` the minors;
  - they come from the system with the system flag, and from the fleets'
    ships with the fleet flag;
  - each counts with Force of at least GNPRTB 3586 (60), free, and of the
    other side (`FUN_00588840`).
- Picks (`FUN_0058a9e0`, `FUN_0058ab00`) take Luke, then Vader, from the
  first set; else a random one of it (`draw(0..count-1)`); else of the
  second set.
- Each defender meets members until it holds an `Encounter` key:
  - the chance is force + force + GNPRTB 3587 (-100), when both Forces
    are non-zero, and only a positive chance draws (`FUN_0058a3f0`,
    `FUN_0055e4d0`);
  - a hit detects the mission unless the member is a decoy;
  - the hit also starts the encounter (`FUN_0055ff60`): both keys and the
    day, and observation 6 for both sides at the system;
  - both are then taken out of the sets.
- In the outcome, a detected member holding an `Encounter` key confronts
  its partner (`FUN_00589360` -> `FUN_00560200`) instead of meeting a
  random defender. It does so only when each names the other on the same
  day.
- The duel:
  - the member rolls RLEVADTB at its effective combat less the partner's
    (`FUN_005349e0`);
  - a member that roll did not injure takes `FUN_0053ea70`, a chance of
    max(GNPRTB 2571, partner - member) and an injury of rand(chance) +
    rand(2573) + 2572;
  - the partner takes the same roll the other way;
  - each side that injured the other raises its base combat by GNPRTB
    2578;
  - the call order is checked in the disassembly at `0x560386` and
    `0x5603dd`.
- Both keys and both days then clear.
- Luke against Vader:
  - the first time, Luke learns `KnowsHeritage`;
  - if he is also free with Force below GNPRTB 3077 (80), he is wounded
    by rand(3080) + 3079;
  - free, he gains max(3084, gap x 3072 / 100) `ForceExperience`.
- Luke against the Emperor: max(3085, gap x 3073 / 100).
- Leia, free and Force aware: against Vader max(3086, gap x 3074 / 100),
  against the Emperor max(3087, gap x 3075 / 100).

## Port

- `mission_detection.rs`:
  - `Run::force_detect` runs before betrayal;
  - `force_defenders`, `force_members` and `pick` build and walk the sets;
  - `begin_encounter`;
  - `evade_or_capture` (the RLEVADTB step, shared with `expose`);
  - `confront` and the Jedi steps in `lessons` and `learn`;
  - `expose_team` sends members holding a key to `confront`.
- `Character::encounter` and `encounter_day` (save v74).
- New effects, applied by `apply_advance_effects`:
  - `MissionEffect::EncounterBegan`, `EncounterEnded` and
    `HeritageLearned`;
  - `SkillRaised` and `SystemIntelligenceGathered`, which the detection
    now also emits.
- Telemetry: `force_encounter`, `confrontation` and `heritage_learned`.
- port:
  - the run applies its effects after it ends. It keeps the keys it set or
    cleared, and the Force its members gained, for the rest of the run.
  - The loader keeps a character's record index without the family byte
    (Luke is `0x242`). `character_family` therefore reads `is_major` when
    the family is 0. The index order is the key order of the shipped
    records.
  - The same fix puts loaded major characters before minor ones in the
    exposure order. Before it, every loaded character counted as a major
    one.
  - The observation reveals the system, as Espionage's does.
  - `FUN_004f9510` and the `0x360` messages are not ported.

## Tests

- `mission_detection.rs`, 28 new tests:
  - Force detection: the pairing and day, the observation, detection,
    and the confrontation that follows;
  - the filters: Force 60, the sum above 100, captive, same side and
    killed;
  - the flags: fleet and system;
  - Force training counts toward a defender's Force;
  - minor characters defend and are detected;
  - a decoy is not detected;
  - Luke is picked first;
  - a defender that already holds a key meets only one member;
  - a zero chance draws nothing;
  - the confrontation needs a mutual key and the same day;
  - the RLEVADTB difference, and the duel's injury and combat raise;
  - Luke's heritage, wound and Force gain, with the evade's Force first;
  - no wound once Luke knows his heritage, or at Force 80;
  - the wound read from GNPRTB 3079 and 3080;
  - a captured Luke learns no Force;
  - the Emperor's lesson, Leia's lesson (aware or not), and the least
    gain;
  - the world applies the keys and the heritage;
  - loaded characters without a family byte.
- `integrator.rs`: `the_force_encounter_confrontation_and_heritage_are_recorded`.
- Shown failing without the change:
  - with no Force detection and no confrontation, 12 of the first 16 new
    tests fail. The other 4 pass either way: 3 filter tests that check
    nothing happens, and the effects' application, which runs on its own.
  - With the family byte read as before, both loaded-character tests
    fail.
  - The tests added after the first mutation run each kill a mutant that
    survived it.

## Campaigns (`playtest-encounter.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1, 2, 3, 7, 13, 42 and 47,
1200 days.

- Luke meets Vader 5 times on seed 2 (days 716 to 1135) and twice on seed
  3 (days 488, 589). The Emperor meets Luke twice on seed 47 (days 887,
  890).
- Every meeting is followed by a confrontation the same day:
  - Luke takes 34 to 123 injury against Vader;
  - on seed 2, day 820, he evades, gains 1 Force, and learns 9 from the
    gap;
  - against the Emperor he learns 13, then 11.
- No `heritage_learned` was recorded. The invented story event "Final
  Battle Imminent" (`0x396`) sets Luke's heritage when Dagobah ends (days
  404 to 610), always before his first meeting with Vader. Removing it is
  the next stage.
- Before the family fix, no campaign had a single encounter: the member
  sets were always empty.

## Browser

- The Fleet window gate passes all 48 cases (interface WASM
  `7926f6ff3abd638245651b1b77b3824a4d3fa1e5eefdb05b1222ca85dd543780`).
  Its first run failed two Alliance cases (move-and-carry, join-fleet) on
  timing, while a mutation run loaded the machine. The rerun passed all
  48.
- The release build ran with `agent-browser` (headless Chromium, 640 by
  480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 25.0 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 3.1 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- The screenshots show original artwork, so they are not committed
  (`target/acceptance/encounter/`):

```
f5c0796cf13ff14b29beb7ae520b651968d6681d6e4b0de51cf00f03a38b21b2  1-veryslow.png
a14185f6f37594b791acb8d82ef01b244ad1519881b6fa95179cb1ef65b3c80e  2-fast.png
```

- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2356 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v74, `Encounter` and
  encounter day):
  - initial `v1:7de82effa8207c75` -> `v1:57115a2a37530826`;
  - final `v1:ab2d9eddde745c32` -> `v1:e6bb717f2403b483`;
  - two recordings are identical, and only the fingerprints changed in the
    manifest;
  - the golden still holds after the family fix;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `a4167160bb67d33ff3c7f65d7e7e394003752dac3704316151f0520f5081c757`.
- `cargo mutants --in-diff` over the core changes:
  - first run: 129 mutants, 20 missed. The tests on the filters, flags,
    minor characters, the zero chance, the RLEVADTB difference, the wound
    parameters and the loser's combat close them.
  - second run: 129 mutants, 118 caught, 6 unviable, 4 timeouts, 1 missed:
    - the 4 timeouts are `pick` mutants that never run out of picks, so
      the run hangs;
    - the 1 survivor is `gap > 0` -> `gap >= 0` in `learn`. It is
      equivalent: a gap of 0 gives max(least, 0) = least.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021) on the changed lines:
  - `run` takes 8 arguments with the day, under an `expect`;
  - `first_pass` stays over 100 lines, as before;
  - nothing else.

Hashes of the committed files are in `SHA256SUMS`.
