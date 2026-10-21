# The AI rates a ship by its recharge after hits (F-032, D4)

Date: 2026-10-21. The reading is in `ghidra/notes/ship-subsystem-damage.md`,
"Recharge after hits".

## Change

- `ai_records::ship_record` (`FUN_00484630`) scales the class's attack
  (`values[0]`, `values[1]`) and shield recharge (`values[3]`) figures by
  the ship's recharge over the class's (`FUN_004738d0`, `FUN_004738f0`,
  `FUN_00473910`).
- The ship's recharge is `rate - hits x rate / GNPRTB 2048` (4): slots
  `+0x264 - +0x290` for weapons and `+0x268 - +0x294` for shields
  (`recharge_after_hits`).
- An undamaged ship's figures are unchanged.

## Campaigns

14 campaigns change (`campaign-totals.txt`):

- battles 2025 -> 1855;
- Alliance wins 643 -> 608;
- Empire wins 1382 -> 1247.

The AI now rates a ship with weapon or shield hits as weaker.

## Gates

- 1 new test. It fails with the change neutralized.
- Workspace 2570 passed, 0 failed.
- Mutants 18 caught, 0 missed.
- `provenance-scan` 0 uncited.
- Browser, Arch Chromium 155.0.8059.84: Fleet window 58 of 58; replay
  equivalence passed; frame timing p99 0.8 / 1.6 ms, 0 frames over 50 ms.
- Release WASM: `cdb545b86e78462a3d9d3eb3c9d68f9df04a9fc9ef29b4027d38ed624c24559c`.
