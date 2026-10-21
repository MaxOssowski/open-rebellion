# Repair mends subsystem hits (F-032, D3)

Date: 2026-10-21. The reading is in `ghidra/notes/ship-subsystem-damage.md`,
"Repair".

## Change

- A ship is Damaged with hull damage or any hit counter (`FUN_00501c00`):
  `repair.rs` arms its timer, and `ai_records::ship_damaged` counts it.
- Each repair fire (`FUN_00501c50`, `repair::repair_hit`):
  - mends a tenth of the hull as before;
  - then, on a roll of 0..99 under the class's `damage_control` (class
    `+0xd8`), draws `r` in 0..n-1 over the n hits;
  - walks the counters in bit order and lowers the first above `r` by one.

  Every hit is equally likely. `RepairEvent::ShipRepaired::hits_after`
  carries the result to the integrator and the app.
- `port:` a fire's draws come from a `SplitMix64` stream seeded by the day
  and the ship's id; the original uses its game generator (`FUN_005f5700`).

## Campaigns

All 24 campaigns change (`campaign-totals.txt`):

- battles 2146 -> 2025;
- Alliance wins 733 -> 643;
- Empire wins 1413 -> 1382.

The first difference in seed 3 is on day 48: a ship with only subsystem
hits starts repair. In seed 10 the AI regroups a ship it now sees as
damaged.

## Gates

- 5 new tests:
  - the weighted pick with its two draws;
  - the `damage_control` gate;
  - repair of a ship with only hits;
  - Damaged by hits;
  - the integrator's write;
  - the draws pinned to SplitMix64's reference outputs.

  With the change neutralized, 3 fail. Workspace 2569 passed, 0 failed.
- Mutants:
  - The first run left 13 survivors: 12 in the unpinned draw mixing, closed
    by pinning; 1 a redundant hull guard, removed.
  - Final: 38 caught, 0 missed.
  - The campaigns after these edits equal the first run, 24 of 24.
- `provenance-scan`: 0 uncited after the port draws were tagged.
- Browser, Arch Chromium 155.0.8059.84: Fleet window 58 of 58; replay
  equivalence passed; frame timing p99 0.8 / 1.7 ms, 0 frames over 50 ms.
- Release WASM: `592511bb1c5621d62c7808283148438a50fa173784e801ceb15e9226d521f079`.
