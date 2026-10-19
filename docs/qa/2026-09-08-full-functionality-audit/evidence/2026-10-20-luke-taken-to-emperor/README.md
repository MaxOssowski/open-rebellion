# Luke taken to the Emperor (F-038)

Date: 2026-10-20.

This is the parity tracker's item "Luke zabrany do Imperatora", in the area
"Zdrada i fabuła". It was moved there from "Postacie i Moc" on 2026-10-19.

## The original

"Luke Discovers His Heritage" (`FUN_0048dc50`) reports a captured Luke
with `0x71a2`, "Luke has been captured by Vader and taken to the
Emperor.", or else `0x71a3`, "Luke has been captured by Vader.".

The test was read from the image (`0x0048dda0..0x0048de28`). The
decompile drops `this` there. `0x71a2` is chosen when:
- Luke and Vader, in the Empire's view (`FUN_004f3e50(2)`,
  `FUN_004f3eb0(2)`), hold the same mission key (`+0x68`,
  `FUN_0042d170`);
- that mission (`FUN_004f3000`, families `0x40..0x7f`, orders included)
  has as its target container `+0x74` the Emperor's container:
  `mov ecx, [ebx+0x1c]`, the Emperor's `+0x1c`, through `FUN_004025b0`;
- the Emperor exists (`FUN_004f3f10(2)`).

The Final Battle's DarthToEmperor hook (`FUN_0054adf0`) is such a
mission: it moves Vader with the captive Luke to the Emperor's location by
a Move order. The earlier reading of the test ("a mission whose `+0x74` is
Luke") was wrong. `+0x74` is a mission's target container
(`mission-lifecycle.md`).

## Port

- `story_messages::taken_to_emperor`:
  - some active mission holds both Luke and Vader (team, decoys or
    captured);
  - its target container is the Emperor's: the abode when it is aimed at
    one, else the system against the Emperor's system or his fleet's;
  - the Emperor is alive.
- `heritage_message` gives `0x71a2` for `0x71a3` when it holds.
- `EncounterNotes::messages` now takes the `MissionState`.
- `final_battle.rs` already dispatches the DarthToEmperor Move order.

## Tests

- `story_messages.rs`:
  `a_captive_luke_on_vader_s_mission_to_the_emperor_is_taken_to_him`. It
  covers:
  - a mission elsewhere;
  - one without Luke, and one without Vader;
  - one to the Emperor's system;
  - abodes;
  - a dead Emperor.
- It fails when the line is forced to `0x71a3` (`fail-without.txt`).
- `cargo test --workspace`: 2466 passed, 0 failed.

## Simulation

Only the message changes, so the campaigns were not re-run. The seed-42
golden is unchanged.

## Gates

- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `9f32c8dba61c54ec48bc6fb76cf7067c71c92eb97ee2715466c22f378ac9219d`).
- The Fleet window gate passes all 48 cases (interface WASM
  `e525747fb508173ddba7b125fa1cfd815eb07467ea7f2c7d8da0a7e7c97d7009`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.5 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.7 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-render`):
  - the first run: 14 mutants, 10 caught, 2 unviable, 2 missed;
  - after the cases without Vader and with abodes were added: 14 mutants,
    12 caught, 2 unviable.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
