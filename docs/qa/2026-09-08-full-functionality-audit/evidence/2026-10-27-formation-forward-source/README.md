# A formation's forward is its side's vector (P24)

Date: 2026-10-27. This is part of the review of the remaining `hyp:` tags.
Two tags are settled by static reading of REBEXE.EXE. No behaviour
changes; only the comments change.

## The original

- `FUN_005ea030` builds a mover. It stores the id it is given (`+0x100`)
  and faces (0, 0, 1) for an id below 1,000,000, else (0, 0, -1):
  `if (param_2 < 1000000) { [9] = 1.0; } else { [9] = -1.0; }`.
- The battle's ids are counted per side:
  - `FUN_005a6290` seeds two counters, 0 and 1,000,000
    (`uVar1 = uVar1 + 1000000` while below 2,000,000);
  - `FUN_005a6620(side)` raises the side's counter, and `FUN_005a6660`
    takes the unit's or force's side index (`+0x168`, `+0x50`, `+0x44`).
- Side 1 is the Empire: `FUN_005afe40` tries the Death Star only for side
  1. Side 0 is the Alliance (`FUN_005be160`, class `0x66c7a0`).
- So a force or group faces +z for the Alliance and -z for the Empire,
  which is the side vector (`original_side_vector`).
- Fighter movers use the same constructor: `FUN_005c62b0` calls
  `FUN_005c72b0`, which calls `FUN_005ea030`.

## The port

- `tactical.rs` `sync_force_movers`: the comment now cites the ids.
- `tactical.rs` `sync_fighter_movers`: the comment now cites
  `FUN_005c72b0`.

## Gates

| Gate | Result |
|---|---|
| `cargo check -p rebellion-core` | clean |
| `provenance-scan check` | 0 uncited items, within the baseline |

Only comments changed.
