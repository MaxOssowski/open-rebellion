# The GID marks follow the original's bands (P07)

Date: 2026-10-22. The reading is in `ghidra/notes/gid-markers.md`
(`FUN_0042b330`, `FUN_00426e70`).

## Change

- `gid_metric_size` now follows each display's thresholds as
  `FUN_0042b330` passes them to `FUN_00426e70`:
  - Available Raw Material (`0x52`): 1 medium, 2 large, 3 or more
    largest. It used the 6/3/1 bands.
  - Idle Shipyards, Training Facilities and Construction Yards
    (`0x65..0x67`): the original's value is 15 when the player's system
    has an idle area of that kind, else 0, so any idle yard is the
    largest mark. The port counted facilities in bands.
  - Idle and Active Personnel (`0x43`, `0x44`): one person is the
    largest mark.
  - The other displays' bands were already the original's; the test now
    cites them.
- `gid_personnel` counts the people as the original does:
  - the player's characters and special forces on the planet;
  - every character aboard the fleets the system holds, the player's
    and the enemy's;
  - busy (Active) means on a mission, captive, injured or holding a
    command.

  The mark takes the player's colour when the player counts anyone,
  else the enemy's, else the holder's. The port counted only the
  player's characters, and only those not on a mission.

port: a killed character counts as not usable. The port's fleets carry
no special forces. The fleets a system holds include those en route to
it (`movement::listed_fleets`).

Open: the fleets' both-sides art `0x27b0` and the fleet displays'
colours, display `0x61`, and the `0x13..0x17` and `0x91..0x93` displays.

## Gates

- Tests: 1 new, 1 strengthened (no longer "no recovered source").
  Workspace 2645 passed, 0 failed.
- Mutants: 72 caught, 1 unviable, 0 missed after the test gained the
  holder and the third-person cases.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build),
  the GID scenarios, 4 of 4 cases each: personnel, active-personnel,
  raw-material, idle-shipyards, idle-training, idle-construction and
  popular-support.
- Replay equivalence passed.
- Release WASM `eeaaf980892331028285156cc688d958a22e04aa3ba03b425286f86b3eb87317`
  (built before the last test-only change).
