# Character Discovers Traitor (F-038)

Date: 2026-10-18.

Story message row 17, the last unmapped class
(`ghidra/notes/story-message-classes.md`, "Row 17").

## The original

- In the betrayal phase (`FUN_00589f10`), the first disloyal member
  betrays the mission. Each other member (`FUN_00589490` ->
  `FUN_00588e80`) then learns the traitor with a chance of its Force
  `+0x8c` (`FUN_0055e550`). Its TraitorDiscovered key `+0xa0` becomes the
  traitor (`FUN_004ee5c0`), and slot `+0x3b8` (`FUN_004f2090`) posts
  `0x361` with both keys.
- `FUN_0048c9c0` shows the discoverer's side "{1:1} Discovers Traitor" /
  "Through the use of the Force, I have discovered that {2:1} has
  betrayed us to the {3:1}.", on its side background with the
  discoverer's portrait.

## Port

- `mission_detection::betray` keeps the betrayal's draws. After a
  betrayal it rolls each other character member's Force and reports
  `MissionEffect::TraitorIdentified`.
- The app posts `story_messages::discovers_traitor_message`. The
  telemetry records `traitor_revealed` with the traitor. The fixture
  posts Leia discovering Chewbacca.
- port:
  - only characters learn;
  - `+0xa0` is not stored;
  - the side names are "Alliance" and "Empire".
- No save format change. The seed-42 golden is unchanged.
- All 21 story message classes are now mapped.

## Tests

- `mission_detection.rs`: `the_other_members_learn_the_traitor_on_their_force`
  (Force 100 learns, Force 0 does not, the traitor draws nothing, and no
  betrayal means no learning). It fails with the learning removed
  (`fail-without.txt`).
- `story_messages.rs`:
  `a_traitor_discovery_names_both_and_the_other_side_for_the_discoverer_s_side`.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:

- 41,805 missions dispatched;
- none had two characters on its team, so no one can learn a traitor in
  AI play. A player's multi-character team can.

## Browser

- The story-message fixture (Alliance, code 312), checked with
  `agent-browser`: "Leia Organa Discovers Traitor" / "Through the use of
  the Force, I have discovered that Chewbacca has betrayed us to the
  Empire.". Background `0x412` (1042), Leia's portrait (6209).
- Screenshot not committed (original artwork):
  `a53f1f392d916f45fc7ef3de5142904846f5a5b2f50e2eacf1b7390bc11c289a
  target/acceptance/story-messages-2/discovers-traitor.png`.
- The Fleet window gate passes all 48 cases (interface WASM
  `4073418e5d6ccd579e8e9eec5e46146807409d242ffba5d656c3420f6f21986e`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.7 ms | 25.3 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 3.6 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.

## Gates

- `cargo test --workspace`: 2409 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Replay equivalence passes with the unchanged golden
  (`v1:e8ceaa386f41cc96` -> `v1:475c0c0f35968fba`) and 0 browser errors.
  Release WASM SHA-256
  `ce05e85a1b4f208cc95fb832aa5493d90c8c88b2a9d974b0d7ffc0a2f3d66154`.
- `cargo mutants --in-diff` (`mission_detection.rs`): 8 mutants, 8
  caught.
- `provenance-scan check`: 88 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
