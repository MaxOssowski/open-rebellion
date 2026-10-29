# The character messages' advisor events (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy". This completes the codes
left uncoded by `../2026-10-29-advisor-events/`. The reading is in
`ghidra/notes/advisor-briefing.md`, "The events".

## The original

**The speaker map.** `FUN_0048db30`, and its copies in the builders below,
reads a speaker:
- a major character gives its own code: `0x240` Mon Mothma `0x17`, `0x241`
  Leia `0x18`, `0x242` Luke `0x16`, `0x243` Han `0x19`, `0x280` the Emperor
  `0x1a`, `0x281` Vader `0x1b`;
- a speaker that is no character (family outside `0x30..0x37`) gives
  `0x15`;
- any other character sets no code.

**The capture map.** `FUN_00490340` (`0x1e0`, cases 3..5) maps Luke
`0x1e`, Mon Mothma `0x1f`, Leia `0x20`, Han `0x21`, the Emperor `0x22` and
Vader `0x23`; anyone else gets `0x15`.

Each class's kind (vtable slot `+0x10`, read from REBEXE) is a delivered
kind:
- 3: the generic getter `0x5a6730`;
- 8: `0x48ad30`;
- the mission report's `+0x84`: 3 or 5, set in `FUN_004927c0`.

The traitor, rescue and Dagobah classes were found by their vtables
(`0x65b2f0` kind 3, `0x65b2a8` kind 8, `0x65b380` kind 8). The same reading
confirms the kinds of the classes coded in the previous stage.

## The port

- `story_messages::speaker_event` / `character_speaker_event` and
  `captured_event`.
- Codes now attached:

| Message | Builder | Code |
|---|---|---|
| Mission report / failed | `FUN_004927c0` on its leader `+0x74` | speaker map |
| Character injured / healed | `FUN_0048fdf0` | speaker map |
| Force Growth; Leia Uses Force / Future Jedi | `FUN_0048ed80`; `FUN_0048f1e0` | speaker map |
| Character Discovers Traitor | `FUN_0048c9c0` | speaker map on the discoverer |
| Rescue Attempt | `FUN_0048c4c0` | speaker map on the rescuer (`+0x68`) |
| Luke goes to / leaves Dagobah | `FUN_0048d190`, `FUN_0048d0f0` | `0x16` |
| Jabba Captures | `FUN_00490340` case 5 | capture map |
| Emperor Arrives at Coruscant | `FUN_0048ce80` | `0x1a` |
| Killed by Imperial Assassins | `FUN_0048b050` | `0x15` |
| Jabba Delivers Prisoners | `FUN_0048b6e0` | `0x1b` |
| Bounty hunters | `FUN_0048fa00` | `0x21` captured; else `0x19` (Alliance), `0x2d` (Empire) |

- hyp: case 5 of `FUN_00490340` leaves its switch for Han in a way that may
  drop the message; the port keeps `0x21`.
- Not coded:
  - the port's "has been killed" line (notification `0x1a`): its class
    is not identified;
  - encounters (`FUN_0048d7f0`): they write no code.

## Browser

Regression only: neither gate posts a character message.
- `advisor-events.mjs`: `…T15-31-45-554Z`, test WASM in
  `advisor-events-result.json`;
- `agent-objectives.mjs`: `…T15-30-01-986Z`, release WASM `fc5526b0…`.

The codes are shown by the tests.

## Tests

- `cargo test --workspace`: all pass.
- New test: `the_majors_speak_by_their_own_codes_minors_by_none_and_others_by_0x15`.
- Added assertions, with no expectation weakened: the Emperor's,
  Jabba's-capture and bounty messages' codes.
- `cargo mutants --in-place --in-diff` on `story_messages.rs`: 41 mutants,
  30 caught, 11 unviable. The two app sites are in the untested main loop.
