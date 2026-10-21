# The tactical battle simulation moves to rebellion-core (F-033)

Date: 2026-10-21. Stage W2c, step one, of the "Walka" area. The user
agreed to the architecture change on 2026-10-21.

## Why

`ghidra/notes/battle-flow.md`: the original resolves an abstract battle
by stepping the tactical module without display (`FUN_0059a530` ->
`FUN_005ae9c0`, step 1.0, stalemate 1200). For the port to do the same,
its tactical simulation must be in `rebellion-core`, not in the render
crate.

## What moved

- `crates/rebellion-core/src/tactical.rs`: `BattleSession` with `step`,
  the ships, fighters, Death Star, beam, RNG, damage, subsystem, order,
  weapon-effect and voice-cue types, and their helpers (172 items,
  copied unchanged and made `pub`).
- `crates/rebellion-core/src/tactical_resources.rs` (renamed from the
  render crate, unchanged code).
- `crates/rebellion-core/src/pictures.rs`: `planet_picture_id` and
  `character_mini_resource_id`, which the battle state and the windows
  both use.

## What stays in rebellion-render

- `tactical_view.rs`: drawing, input, the HUD, and the battle windows.
  It re-exports the moved names, so `rebellion_render::tactical_view::*`
  paths still work.
- The three display-only methods are render-side traits now:
  `TacticalWorldPosition::rendered` (Vec3), `TacticalOrder`'s HUD
  bitmaps, and `WeaponEffect::color`.

## The one structural change

`BattleSession` called `macroquad::logging::info!` twice: when the
superlaser is committed and when it is resolved. Core has no macroquad.
The lines now go to `BattleSession::diagnostic_log`, and the view prints
them right after each call that can produce them: `step`,
`advance_presentational_effects`, and `commit_death_star_shot`.

The first gate run printed only after `step`, which delayed the
"resolved" line by one step. The laser-journey harness screenshots on
that line, so the "resolved" frame changed. Printing after
`advance_presentational_effects` too restored it (`console.txt`).

## Provenance

The scanner covers `rebellion-core`, so it now saw the moved code: 100
uncited items. Each now carries its source:

- a `FUN_`, taken from `ghidra/notes/tactical-*.md`, the decompile, or the
  voice map (`docs/reference/asset-library/tactical-voice-map.json`);
- `port:` for the 2D fallback layout (`ARENA_*`, `SHIP_SPACING`,
  auto-placement), the port's four steps a second, and the escort
  offsets;
- `hyp:` where the writer is not located: the `FUN_005ec400` layout
  constants, the Surround/Stand Off codes, and the GOKRES bands.

The result: `ok: 0 uncited items`.

## Gates

| Gate | Result |
|---|---|
| Workspace tests (fixture feature) | 2514 passed, 0 failed, 44 ignored |
| 24 campaigns vs the 5120 stage (f43) | 24 of 24 identical |
| Replay manifest | 2 passed |
| seed 42 golden | initial `v1:e01dc3b9b044c28e`, final `v1:7737eb96955b5cd7` (unchanged) |
| provenance-scan | 0 uncited |
| Browser equivalence and perf | passed, 0 browser errors |
| Fleet window suite | pass, 58 cases |
| Status window | pass |
| Tactical Death Star scenarios (4) | 4 of 4 pass |
| Screenshots vs the last pre-move run | 63 of 64 byte-identical (`screenshots.SHA256SUMS`) |
| Release WASM | `9b344cfdc1afed9dd91c92e8f85c405ccff73d8981eb947dbcab31f6efad3163` |

The screenshot that differs is the 640×480 `death-star-superlaser-beam`
frame. It captures the beam in flight, and it had a different hash in
each of the last five runs before this change (2026-10-20 19:53, 19:55,
20:00; 2026-10-21 02:32).

## Mutants

`cargo mutants --in-diff` on `tactical_view.rs` (`mutants-missed.txt`):
126 mutants, 48 caught, 2 unviable, 76 missed. Every surviving mutant is
in display code that no unit test covers:

- the log printer;
- the moved render-side traits (Vec3, HUD bitmaps, colour);
- `TacticalCanvas`, the `BattleAlertTab` label, and `handle_combat_input`.
  These lines did not change. The large deletion diff re-lists them.

The browser suites above exercise this code. No simulation line changed,
so the moved simulation keeps the tests it had in the render crate, and
the 24 campaigns did not change.
