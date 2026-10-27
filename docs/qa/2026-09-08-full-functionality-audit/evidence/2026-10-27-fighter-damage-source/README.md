# A simulated battle's fighter damage reads the fighter's own shield and hull (P24)

Date: 2026-10-27. This is part of the review of the remaining `hyp:` tags.
The abstract battle damages a fighter group through the non-virtual
`FUN_005b54d0`. The port assumed that the function reads a group's shield
and hull at the capital ship's offsets. Static reading of REBEXE.EXE
confirms it. No behaviour changes.

## The original

- `FUN_005b54d0` reads the shield `[0xf0]` (`+0x3c0`) and the hull
  `[0xf3]` (`+0x3cc`):
  - shields first;
  - an overflow below 0 of at most `_DAT_0066c500` goes to the hull. That
    constant reads -1.0 in REBEXE.EXE's `.rdata`, so the hull loses at
    least one point;
  - two draws `FUN_005af420(0, 100)` feed slot `+0xa0`.
- The fighter group's constructor `FUN_005b9c60` runs the shared base
  `FUN_005b0f70` and writes `[0xf3]` (its hull) and `[0xf0]` (its shield),
  the same offsets as the ship's `FUN_005b05c0`.
- `FUN_005d14d0` then tests `+0x3cc` against 0 for any target before it
  calls `FUN_005af7a0`.
- A group's slot `+0xa0` hit changes nothing. Its subsystem events
  (`0x1e`..`0x22`) are posted only for slot `+0x20` = 0 (`FUN_005b5100`).

## The port

The comment in `abstract_battle.rs` (`deal`, the fighter arm) now cites
these sources.

## Gates

| Gate | Result |
|---|---|
| `cargo check -p rebellion-core` | clean |
| `provenance-scan check` | 0 uncited items, within the baseline |

Only a comment changed.
