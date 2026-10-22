# The Assault Summary's forces labels and unit names (F-006, AS5)

Date: 2026-10-22. Compared against the original captures
`docs/qa/2026-09-10-interface-parity-audit/reference-captures/steam-guide/`
guide-061 (bombardment overview), guide-062 (bombardment, System Forces,
Trooper Regiments) and guide-065 (assault overview).

## Change

- `FUN_0044e290` (the window's paint slot) draws, on a forces page, the
  rail button's label at (12, 36), 347 wide, centred, in the attacker's
  colour ("System Forces" in guide-062), and the category button's label
  at (38, 100), white, left-aligned ("Trooper Regiments"). The port now
  draws both.
- Each row shows its unit's name under its picture (`FUN_0044edb0`, item
  `+0x34`), as guide-062 shows "Mon Calamari Regiment" under each
  picture.

## Comparison

`*-assault-system-forces.png` against guide-062: the same title, page
label, category bar, category label, column heads, pictures on the
regiment backdrop and names. Remaining differences: the port draws no
scroll bar, and its font sizes are approximations.

## Gates

- Tests: 3 extended. Workspace 2615 passed, 0 failed.
- Mutants on the diff: 15 caught, 0 missed.
- Browser, Arch Chromium 155.0.8059.84: Fleet window 58 of 58. Interface
  WASM `438c283000a7049529387212183819ba80ad217ada4c45e0fd20215cd517acff`.
- Replay equivalence passed.
- Release WASM `64b309cb7acd66a0e18c5b2107543a577facebb0a7f4deea5068d679563eb6d9`.
