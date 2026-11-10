# A mission report has no "no result" state

Date: 2026-11-10. Covers OBJ-12-C008 (no result), excluded under the
owner's decision of the same day (states absent from the original are
removed). Trace only. No code changed.

## The trace

A mission's per-mission result is `+0x60`, set through `FUN_00521880`,
with values 0..3 (`ghidra/notes/mission-lifecycle.md`, "Phase 10").

- **Phase 8.** `FUN_00524b70` case 8 resets it to 0 before the members
  roll.
- **Phase 10.** Each member's roll (slot `+0x27c`) sets 3 on a success.
- **The world effect** (slot `+0x280`) runs once after the members. Every
  reported mission's handler turns a result still at 0 into 2, a
  failure:

| Handler | Missions | 0 becomes |
| --- | --- | --- |
| `FUN_00576700` (shared) | Recruitment, Incite and Subdue Uprising, Jedi Training, Rescue, Abduction, Assassination, Pickup | 2 |
| `FUN_00574120` | Diplomacy | 2 |
| `FUN_00573610` | Espionage | 2 |
| `FUN_0056bee0` | Reconnaissance | 2 |
| `FUN_005746e0` | Sabotage, Death Star Sabotage | 2 |
| `FUN_0056e870` | Palace | 2 |
| `FUN_00572490` | Dagobah, Vacation | 2 |
| `FUN_0056cf30` | the design researches | 2, or 3 with points (`(RdPoints != 0) + 2`) |

The only other setter is the bounty hunters' story mission
(`FUN_00575de0`, 3 when Han is captured). Its `+0x280` is the no-op
`51ebb0`, and it files no mission report.

No code sets 1. So a report reads success (3), failure (2), or the end
codes foiled and aborted (`+0x64`). These are OBJ-12-C004..C007, which
all pass. There is no "no result" outcome. Recruitment with an empty
pool, the one roll that leaves 0, also ends as a failure.

The port's mission reports have no such state either (the cell's
earlier note).
