# Rescue Attempt (F-038)

Date: 2026-10-18.

Story message row 13 (`ghidra/notes/story-message-classes.md`, "Row 13,
Rescue Attempt").

## The original

- `FUN_0048c4c0` answers notification `0x200` (`RoleEnrouteActiveNotif`,
  `FUN_005365b0`).
- It is shown when all of these hold:
  - the view is the Alliance's;
  - the character is of family `0x30`..`0x3b`;
  - the new value is set;
  - the record's second key (`+0x2c`) is of family `0x64`, a Palace
    mission. Ghidra dropped this test as unreachable; the disassembly at
    `0x48c5d9`..`0x48c610` has it.
- It shows TEXTSTRA `0x71ea` "{1:1} Rescue Attempt" / `0x71eb` "Han Solo
  has been captured by bounty hunters and taken to Jabba's palace. I will
  be departing immediately to attempt a rescue.", on background `0x412`
  with the rescuer's portrait (`FUN_004c5000`).

## Port

- `story_messages::rescue_attempt_message`.
- The app posts it when a `JabbaStep::Rescue` dispatches the Palace
  mission. The story-message fixture posts it for Chewbacca.
- port: the dispatch stands in for the rescuer's EnrouteActive (the
  original sets it as the rescuer leaves). The speaker and media are not
  read.
- 1 of the 21 story message classes remains unmapped: 17, Discovers
  Traitor.

## Tests

- `story_messages.rs`:
  `a_rescue_attempt_names_the_rescuer_for_the_alliance_with_his_portrait`.
  The function is new, so the test cannot pass without it.
- The app's wiring has no unit test. It is checked in the browser.

## Browser

- The story-message fixture (Alliance, code 312), checked with
  `agent-browser`: "Chewbacca Rescue Attempt" and the body above,
  background `0x412` (1042), portrait 6723.
- Screenshot not committed (original artwork):
  `50e0aa91949cc45fdd2e67775aca84f6291bf4b24cfcafb98f488f1da785c917
  target/acceptance/story-messages-2/rescue-attempt.png`.
- The Fleet window gate passes all 48 cases (interface WASM
  `bd5f7dff8246ac84b5786d4a45889777f04f459c97d37316ef9411ac9762bf3c`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.8 ms | 25.0 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.2 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.

## Gates

- `cargo test --workspace`: 2407 passed, 0 failed.
- Replay equivalence passes with the unchanged golden
  (`v1:e8ceaa386f41cc96` -> `v1:475c0c0f35968fba`) and 0 browser errors.
  Release WASM SHA-256
  `68b7f0b3f8b59d1c0d3be3d67b7fb120728f5f03d5195ac9d29fb102496ac53b`.
- `cargo mutants --in-diff` (`story_messages.rs`): 2 mutants, 1 caught,
  1 unviable.
- No simulation file changed.

Hashes of the committed files are in `SHA256SUMS`.
