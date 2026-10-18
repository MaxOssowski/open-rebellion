# Emperor Arrives at Coruscant: SeatOfPower (F-038, save v78)

Date: 2026-10-18.

Story message row 16 (`ghidra/notes/story-message-classes.md`). The
port's own `0x230` was a story event that fired once on day 90 with 100
Empire systems, with a text of its own. The original's `0x230` is the
character manager's SeatOfPower. The user approved the save change to
v78 for this stage.

## The original

- `FUN_005617b0` computes SeatOfPower. It is 1 when the Emperor
  (`0x34000280`):
  - has a location;
  - is not captured (`+0xac` bit 0) and is usable;
  - stands in Coruscant itself (his container is `0x90000109`, not a
    fleet);
  - and Coruscant's side (`+0x24 & 0xc0`) is the Empire's.
- `FUN_0055f650` stores it (character manager `+0x58` bit 1). On a change
  it calls slot `+0x1d8` (`FUN_00562450`), which posts notification
  `0x230` with the new value (`FUN_0053fdd0`).
- `FUN_0048ce80` builds Emperor Arrives at Coruscant for a set value, for
  the Empire only:
  - TEXTSTRA `0x71c8` / `0x71c9` "I have returned to the Seat of Power at
    Coruscant.";
  - background `0x42c`, speaker `0x1a`.

## Port

- `seat_of_power.rs`:
  - `SeatOfPowerState` (save v78);
  - `seat_of_power(world)`;
  - `SeatOfPowerSystem::advance`, which returns a change.
- Simulation step 5e and the app run it daily. The telemetry records
  `story_seat_of_power` with the new value.
- The app posts `story_messages::emperor_arrives_message` to the Empire
  when the bit is set. The story-message fixture posts it too.
- `story_events.rs` no longer defines the port's `0x230`. Its two tests
  are replaced by `the_invented_emperor_arrival_is_gone`.
- port:
  - the check runs daily, in place of the character manager's handlers;
  - "usable" has no port field.
- hyp: the first check stores the bit silently. The setup places the
  Emperor at Coruscant (`FUN_0055f7b0`) before play, so the original
  would not greet the player with this message on day 1.
- 4 of the 21 story message classes remain unmapped.

## Tests

- `seat_of_power.rs`:
  - `the_seat_of_power_needs_a_free_emperor_in_an_empire_held_coruscant`:
    captured, killed, in a fleet, without a system, in an abode, an
    Alliance Coruscant, another system;
  - `only_a_change_after_the_first_check_notifies`.
- `story_messages.rs`:
  `the_emperor_arrives_at_coruscant_for_the_empire_on_its_picture`.
- Shown failing without the change (`fail-without.txt`): with the side
  test and the change report removed, both `seat_of_power` tests fail.

## Campaigns (`playtest-seat-of-power.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days.

- In every seed the Emperor leaves Coruscant between days 10 and 15
  (SeatOfPower clears).
- In 23 seeds he returns between 1 and 4 times, so the Empire reads
  Emperor Arrives at Coruscant that often. Examples: seed 1 on days 102
  and 314, seed 8 on days 194, 485, 789 and 1030.
- In seed 11 he never returns.

## Browser

- The story-message fixture (Empire, code 568), checked with
  `agent-browser`: "Emperor Arrives at Coruscant" / "I have returned to
  the Seat of Power at Coruscant.", background `0x42c` (1068).
- Screenshot not committed (original artwork):
  `13671466f2cda1e9f4dcb280c13b2a1acdf69ca87dc0d2ece0a0a98fc2d25129
  target/acceptance/story-messages-2/emperor-arrives.png`.
- The Fleet window gate passes all 48 cases (interface WASM
  `30b0fc3689aa2ff48c21e27da93ffade9c57baccfd691063bb9f9d75380631b4`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 25.4 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 3.4 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.

## Gates

- `cargo test --workspace`: 2401 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v78, the SeatOfPower
  state):
  - initial `v1:5474ae69134d1255` -> `v1:d8f4a9b46e07169c`;
  - final `v1:4cd75bad1e5062b8` -> `v1:fd4bcf87c87325d6`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `760032a9ee89bccf0729014a740e99239111c0c4311316d9272e0fe9a5481dfb`.
- `cargo mutants -f seat_of_power.rs`: 15 mutants, 15 caught.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt on the changed lines: clean, apart from
  the story builders' shared `implicit_hasher`.

Hashes of the committed files are in `SHA256SUMS`.
