# Browser acceptance: an enemy ship is selected and Space targets (P24)

Date: 2026-10-27. Stages `../2026-10-26-enemy-selection/` and
`../2026-10-27-battle-keys/` were checked by unit tests only. This stage
adds two probes to the browser group journey
(`probeTacticalGroupPresentation` in `tools/interface-parity/run.mjs`).

## The probes

- **`source-enemy-capital-selection`.** A left click on the projected
  hostile capital ship. It must log
  `[tactical_3d] selection object_id=<the hostile> `. The frame must
  differ from the one before the click. Source: `0x005a0d10`,
  `FUN_005a1110`.
- **`source-space-nearest-target`.** Key 1 selects task force 1, then
  Space. It must log `[tactical_navigation] event=nearest_target` with at
  least one member. Source: `FUN_0059b730` key `0x20`, `FUN_005aa970`.

## Result

`production-group-presentation`, Alliance and Empire at 640×480 and
1280×800 letterboxed: 4 passed, 0 failed
(`.artifacts/interface-parity/2026-10-27T01-35-10-090Z-1739294`). The
interface-test WASM was the one the s146 gates had just built from the
same Rust tree; only `run.mjs` changed.

The Alliance 640×480 frame (`enemy-capital-selected.png`, SHA-256
`262ed8dcd618381042c41554c4906499aa1b133b5079be92c4275cf59d332aa7`) was
inspected:
- The far Imperial ship (object 9) has the yellow selection frame.
- The selected-ship panel shows "Strike Cruiser", its art, its hull and
  shield meters, and "No Orders" with "Tactics: Stand Off".
- There is no "Task Force #" caption, as the enemy-selection stage
  intended.
- Space logged `members=1`.
