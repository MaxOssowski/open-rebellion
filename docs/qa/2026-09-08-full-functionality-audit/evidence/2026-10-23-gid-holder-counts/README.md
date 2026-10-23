# The counting map displays count the holder's units (P07)

Date: 2026-10-23.

## Source

`FUN_0042b330` sets `uVar16` to the system's `+0x24` bits 6..7, its
holder, and hands it, not the viewer `+0x9c`, to the counters of
`0x53` (`FUN_0052d370`), `0x54` (`FUN_0052cf40`), `0x62..0x64`
(`FUN_0052c8c0`, `FUN_0052c5a0`, `FUN_0052c270`), `0x71`
(`FUN_005044f0`), `0x72` (`FUN_00503550`), `0x73` (`FUN_0051c0d0`),
`0x74` (`FUN_005276d0`) and `0x75` (`FUN_005273d0` + `FUN_00527150`).
The idle yards `0x65..0x67` give their value only when the holder is the
viewer.

## Before

`gid_metric` counted the viewer's units: an enemy system showed the
player's (usually none) in the enemy's colour. The squadron test
expected that from a guess marked untraced.

## Change

`gid_metric` counts the holder's mines, refineries, yards, regiments,
squadrons and defenses; the idle yards count only at the viewer's own
systems. The squadron test's expectation changes with the source above.

## Gates

- Workspace 2696 passed, 0 failed.
  `the_counting_displays_count_the_holder_s_units_whoever_views` fails
  without the change (left 1, right 2).
- Mutants on the touched render code: 19 tested, 19 caught.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- Replay equivalence passed.
- Release WASM `fd5936f79412eae04734cdc1a033c1b5112976150936876e7617c1cd28ab4541`.
