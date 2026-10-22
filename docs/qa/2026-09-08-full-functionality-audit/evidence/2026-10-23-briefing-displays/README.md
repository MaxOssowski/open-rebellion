# The briefing's map displays (P34, P07)

Date: 2026-10-23. Stage 2 of the opening briefing
(`ghidra/notes/advisor-briefing.md`, "The briefing's displays").

## Source

`FUN_0042b330` cases `0x13..0x17`, `0x20`, `0x91..0x93`, which the
briefing's actions select (`FUN_004c0fc0`). The viewer is `+0x9c`, the
holder the system's `+0x24` bits 6..7, the support side `+0x84` bits
2..3 (`FUN_0055a080`, GNPRTB 7760). Names from TEXTSTRA `0x161f`,
`0x1620`, `0x1621`, `0x1629`, `0x162a`.

## Change

- `economy::support_side`, taken out of `resolve_system_control`.
- `GidMode` gains the briefing's displays (not in the GID menu):
  `OwnLoyalSystems` `0x13`, `OwnMilitaryControl` `0x14`,
  `EnemyLoyalSystems` `0x15`, `EnemyMilitaryControl` `0x16`,
  `UnexploredSystems` `0x17`, `BriefingFleets` `0x20` (Idle Fleets'
  case), `AllDefenses` `0x91`, `MarkedImperialSystem` `0x92`,
  `MarkedAllianceSystem` `0x93`.
- Their marks: the largest in the holder's colour when the holder and
  support side match the display, else the smallest; Unexplored Systems
  turns the unexplored gate round; All Defenses sums the holder's
  regiments, armed defenses (families `0x22..0x27`) and squadrons; the
  marked displays mark the map's selected system (`GidOverlayContext::
  marked_system`) and give the others the unexplored mark.
- port: no legend for these displays (the original's is an empty window).
  Nothing selects them yet; the briefing's playback is the next stage.

## Gates

- Workspace 2695 passed, 0 failed.
- Mutants on the touched core and render code: 58 tested; the first run
  left 9 alive (an Empire system under military control, squadrons and
  the holder's side in All Defenses, the names untested). The tests now
  cover them; the last, `facility.side == holder` -> `!=`, fails the
  test by hand after the second run.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- Replay equivalence passed.
- Release WASM `c7dc313166614a5b0c7d6b50cf7b0a758b1b9ab3647281a3efe1ed09b7ad350a`
  (the tests changed after it; the code it builds did not).
