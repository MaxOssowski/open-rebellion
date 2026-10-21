# GNPRTB 5120 no longer scales battle damage (F-033)

Date: 2026-10-21. First stage of the "Walka" area (tracker item
"Usunięcie parametru 5120 z walki").

## The original

- GNPRTB 5120 (`0x1400`, 5 in every column) is the transit distance
  divisor: `FUN_0055d8c0` computes
  `ticks = max(1, isqrt(dx² + dy²) / GNPRTB 5120 × speed / 100)`
  (`build-delivery.md`, `movement.rs`).
- No recovered battle code reads it (audit F-033).

## What the port did before

- `CombatSystem::resolve_space` read it as a "combat difficulty
  modifier", per cent, and multiplied every hull hit by 5 / 100, with at
  least 1 point per hit.
- Its citation (`FUN_0053e190`, `a × b / 100`) is a generic helper, not a
  caller with 5120.

## Port

- The modifier, its constant and the `difficulty` argument it needed are
  gone from `resolve_space` and `resolve_system_space_combat`.
- A hit now takes its full damage from the hull.
- The rest of the auto-resolve stays the port's own (`hyp:`, F-033).

## Tests

- New: `hull_damage_is_not_scaled_by_gnprtb_5120`. With the old scaling
  put back it fails, 16 against 360 points (`fail-without.txt`).
- Removed: `space_combat_difficulty_scales_hull_damage`. It asserted the
  invented scaling.
- `cargo test --workspace`: 2513 passed, 0 failed.
- `cargo mutants --in-diff`: 7 mutants, 4 caught, 3 unviable.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:
- 4 are identical, 20 change;
- summed over all 24:

| | Before | After |
|---|---|---|
| space battles | 960 | 901 |
| battle rounds | 1855 | 1650 |
| capital ships lost | 451 | 401 |
| decisive battles | 730 | 643 |
| stalemates | 217 | 236 |

- Both sides stay alive in every campaign.
- Battles end in fewer rounds. The seed-42 golden has no battle and is
  unchanged.

## Gates (`f43`)

- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `b04dbbd66762ed80f2dd1e03dc45da9c9bccb53b97a769e34ed6e8e9d19c7138`).
- The Fleet window suite passes 58 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- `provenance-scan check`: 0 uncited items.

Hashes of the committed files are in `SHA256SUMS`.
