# The Battle Summary's scene follows the viewer (F-006 / TAC-07)

Date: 2026-10-22. Notes: `ghidra/notes/battle-summary-window.md`.

## Change

- `tactical_view.rs`: `battle_summary_scene` picks the summary page's
  scene as `FUN_0046f360` does, from the viewer, the winner and the side
  that withdrew:
  - no winner: 11619 (`0x2d63`);
  - the viewer's win: the viewer's victory (10759 Alliance, 10760
    Empire), or the enemy beaten off (10758 / 10757) when the enemy
    withdrew;
  - the viewer's withdrawal: the viewer beaten off (10757 / 10758);
  - the viewer's loss: the enemy's victory.
- Before this change the port picked the scene from the winner alone,
  so a withdrawal showed a victory scene.
- `battle_summary_page` takes the scene from it.

## Gates

- Tests: 1 new. Workspace 2644 passed, 0 failed.
- Mutants: 9 caught, 1 unviable, 1 missed: the macroquad wrapper
  `draw_battle_result_summary` as a whole. The screenshots show it.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build):
  tactical `production-battle-results-presentation` passes 4 of 4 cases.
  Both fixtures are the viewer's win with no withdrawal, so the probe
  still verifies STRATEGY 10759 (Alliance) and 10760 (Empire)
  pixel for pixel.
- Replay equivalence passed.
- Release WASM `e7dd553fa87a9adcccf43a2368584fb32ef63dc597d424efd5c5ad4bb23e5e8f`.
