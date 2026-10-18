# The bounty hunters (F-038)

Date: 2026-10-18.

The port's Han and Jabba story was its own chain of story events:
- `0x397` Bounty Hunters Active and `0x212` Bounty Hunter Attack, which
  spawned a special force at Han's fleet;
- `0x398` Han Frozen in Carbonite and its countdown `0x39c`..`0x3a0`,
  `0x39b`;
- the rescues and palace captures `0x380`..`0x387`, `0x399`, `0x39a`,
  `0x200` and `0x231`.

They ran on tick thresholds and random gates. This stage replaces the
start of that story with the original's BountyHuntersActive path, read in
`ghidra/notes/jabba-palace.md`. Plan steps 1 and 2 are done together:
the palace autorouting (step 1) has no caller without the Bounty mission
(step 2). The rest of the invented chain is removed too, because it
depended on the port's own `0x212`.

## The original

- The mission manager's setup sets BountyHuntersActive (`+0x58` bit
  `0x4000`). Its hook (`FUN_0054b740`) arms timer `0x393` for GNPRTB 6148
  (300) + rand(0..=6149 (300)) days.
- The timer fires once (`FUN_00546fa0`). With a chance of GNPRTB 6150
  (100) percent, and Han located, free and usable, it:
  - creates the Bounty Hunters unit (SPECFCSD `0x3c000009`) in Jabba's
    Palace;
  - gives the Empire observation of Han's location;
  - creates the Bounty mission (`0x65000083`) against Han.
- The Bounty hit (`FUN_00575de0`) acts on Han alone:
  - it pulses BountyAttack (notification `0x212`);
  - Han rolls RLEVADTB against the hunters' combat 0 (`FUN_005349e0`);
  - a capture (result 3) clears `CanEscape` and sets
    CapturedByBountyHunters (`+0xb0` bit 2).
- The captive autoroutes. `FUN_00543040` replaces a destination of
  family `0x65` with Jabba's Palace (`0x98000481`).
- `FUN_0048fa00` posts the messages: Han Solo Attacked by Bounty Hunters
  (Alliance) and Bounty Hunters Locate Solo (Empire, with the system), or
  Han Solo Captured by Bounty Hunters (both sides).

## Port

- `jabba.rs`:
  - `JabbaState` (save v77);
  - `JabbaSystem::advance` (the timer and its fire);
  - `perform` (the unit, the observation, the Bounty mission, and Han's
    trip to the palace).
- Simulation step 5d and the app run it after the Final Battle. The
  telemetry records `story_jabba` per step.
- `missions.rs`:
  - `MissionKind::Bounty`;
  - the phase-10 hit: Han only, `mission_detection::rlevadtb_evades`, the
    injury, then `BountyAttack` and, when captured, `CharacterCaptured` and
    `CapturedByBountyHunters`;
  - `autoroute_to_abode`: a 1-day abode trip that ends with
    `MemberEnteredAbode`;
  - a mission's origin abode, so the Bounty Hunters leave the palace.
- World (save v77): `Abode::special_forces` and
  `Character::bounty_captured`.
- `story_messages::bounty_messages` builds the three `0x212` messages, and
  the app posts them after the mission's result.
- `story_events.rs` no longer defines the invented chain (`0x212`,
  `0x200`, `0x231`, `0x380`..`0x387`, `0x397`..`0x3a0`).
- port:
  - the Bounty Hunters unit takes its class's base skills;
  - the mission leaves the palace on a 1-day abode trip, as the Dagobah
    mission does;
  - only the Bounty mission's captive autoroutes. Other captives still
    stay where they are captured (`FUN_00543360` not read).
- Not ported: HanCapturedAtPalace, the Palace missions, Jabba Captures,
  DarthPickup and the Pickup mission (plan steps 3 and 4).

## Tests

- `jabba.rs`, 6 tests:
  - the setup and timer `0x393`'s range;
  - one hunt, for a free, located Han;
  - no hunt for a captured or killed Han, without the palace, or without
    the active flag;
  - only the Bounty mission's captive autoroutes;
  - the unit's skills, its place in the palace and the Empire's view;
  - the roll budget.
- `missions.rs`:
  - `the_bounty_hunters_leave_the_palace_capture_han_and_take_him_there`:
    the capture, the barred escape, the 1-day trip and Han in the palace;
  - `a_han_who_evades_the_bounty_hunters_stays_free`;
  - `the_bounty_hit_acts_only_on_han`.
- `story_messages.rs`: `the_bounty_attack_messages_follow_han_s_capture`.
- `story_events.rs`: `the_invented_jabba_chain_is_gone` replaces the 9
  tests of the invented chain.
- Shown failing without the change: with the palace autoroute and the
  capture flags reverted, the capture test fails (`bh-fail-without`: 10
  passed, 1 failed).

## Campaigns (`playtest-bounty-hunters.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1, 2, 3, 7, 13, 42 and 47,
1200 days.

- The hunters strike on seeds 1, 2 and 3 (days 331, 497 and 561, at
  Yavin, Bothawui and Vagran). Han evades each time, so the mission fails.
- On seeds 7, 13, 42 and 47, Han is assassinated before the timer fires
  (days 119 to 371). On seeds 1, 2 and 3 he is assassinated later (days
  348, 521 and 621). How often the AI kills Han is outside this stage.
- No campaign shows a capture.

## Browser

- The story-message fixture lists the `0x212` messages (checked with
  `agent-browser`):
  - Alliance (code 312): "Han Solo Attacked by Bounty Hunters", background
    `0x41c` (1052);
  - Empire (code 568): "Bounty Hunters Locate Solo", naming Yavin,
    background `0x41e` (1054).
- Screenshots not committed (original artwork):
  - `7914e06194aa5604674403ca26ba10cfc671474698b8d0307e938187d3a63be6
    target/acceptance/bounty-hunters/attacked.png`;
  - `bacd98a83cb800c6e302b1e8100af722b60032524dc1424f35cc314cef06e772
    target/acceptance/bounty-hunters/located.png`.
- The Fleet window gate passes all 48 cases (interface WASM
  `84a4c5a35019c6e37413a7cf36ad331388afc32bbf59242b46d985407d180118`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.8 ms | 25.1 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 3.3 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- Native GUI acceptance was not run for this change.

## Gates

- `cargo test --workspace`: 2381 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v77, the Jabba state and
  its daily rolls):
  - initial `v1:978a2af596955f63` -> `v1:5474ae69134d1255`;
  - final `v1:8c0677e665d1d127` -> `v1:4cd75bad1e5062b8`;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `650befbb626441ed193c1252c13905efb7c1384db47f634831503e3c6c096a84`.
- `cargo mutants --in-diff` (`jabba.rs`, `missions.rs`):
  - first run: 68 mutants, 5 missed. They are closed by tests of a killed
    Han, an inactive flag with a due timer, the 1-day trip, and a non-Han
    target. A redundant guard in `standing` is removed;
  - second run: 66 mutants, 61 caught, 5 unviable.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt on the changed lines: clean, apart from
  the `implicit_hasher` the story builders share, the integrator's
  `BountyAttack` arm (its comment differs from the identical arm), and a
  test's `assert!(… .is_empty())`.

Hashes of the committed files are in `SHA256SUMS`.
