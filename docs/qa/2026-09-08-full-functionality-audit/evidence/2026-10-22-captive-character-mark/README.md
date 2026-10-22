# A captive character's mark (UI, P08)

Date: 2026-10-22. The reading is in `ghidra/notes/captive-escape.md`,
"The captive mark", and `ghidra/notes/fleet-window.md`.

## Change

`FUN_0042c3b0(.., 0, 1)` blits a captive character's (`+0xac` bit 0) GOKRES
`(class & 0xfff) + 0x7000`, the mini plus `0x3000`, over its mini, after
the mini. The Fleet window's Personnel tab and the Defenses window's
personnel page now draw it (`captive_mark`). GOKRES `0x7840..0x7a9b` are
keyed.

## Gates

- Tests: 2 extended. Workspace 2614 passed, 0 failed.
- Mutants on the window diff: 5 caught, 0 missed, 3 unviable.
- Browser, Arch Chromium 155.0.8059.84: Fleet window 58 of 58, sector
  quadrants 12 of 12. Interface WASM
  `50885c115e9d8eb2ff1bd4715e0fba453d358f95ab7c4c910c716855de636fb9`.
- Replay equivalence passed.
- Release WASM `fafd6ec6913634796918500910742f5bea8223eefbaa1c83ad944dc3c6d32d73`.
