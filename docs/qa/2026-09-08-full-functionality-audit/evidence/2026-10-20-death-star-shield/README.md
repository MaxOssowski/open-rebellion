# The Death Star shield bars the fighters' attack, not hull damage (F-017, save v96)

Date: 2026-10-20. A follow-up to the "Gwiazda Śmierci" area (F-017's
open shield clause), on the user's go-ahead.

## The original

- The Death Star shield is DEFFACSD record 4: family `0x25`, TEXTSTRA
  8832, Empire only.
- The manual (p. 124, and the facility list) says it is the only way to
  protect a Death Star from attack: "a foolproof defense against a Death
  Star being attacked by fighters".
- The tactical setup `FUN_0040bcd0`, beside `FUN_0040a700`:
  - counts the battle system's usable shields (`FUN_0051c270(system, 1)`,
    family `0x25`, no side filter);
  - passes the answer through `FUN_00595ef0` to `FUN_005af4e0`, which
    stores it at the tactical Death Star's `+0x70`.
- `FUN_005ba450` is the operational test (`FUN_005ba420`) plus
  `+0x70 == 0`. Every Attack Death Star check calls it: the mission
  control `FUN_005ad930`, and `FUN_005afcc0` and `FUN_005d14d0` in the
  tactical AI.
- Outside the tactical code, the shield checks are called only by the
  interface and the AI. No strategic battle code reads a shield.
- Notes: `tactical-death-star-path.md`, "The Death Star shield". That
  section also corrects `+0x74`: at creation it means "not completed".

## What the port did before

- `DeathStarState::shield_generator_active` (default true) made a Death
  Star immune to hull damage in an auto-resolved battle. It applied only
  to classes of family `0x34`, the Empire major characters (F-025), so no
  shipped Death Star was affected. It had no source, and nothing could
  turn it off.
- The tactical Attack Death Star ignored any shield on the system.

## Port

- `TacticalDeathStar::shielded` (`+0x70`) is set from the battle system's
  family `0x25` facilities.
- `attackable()` is `FUN_005ba450`.
- `attack_death_star_enabled` is `FUN_005ad930`'s test. Both the click
  handler and the drawing use it.
- The auto-resolved battle's immunity is removed:
  - the `resolve_space` and `resolve_system_space_combat` parameter;
  - `ShipSnap::is_death_star` and the hull-damage branch.
- So is the whole strategic `DeathStarState`: save, simulation, replay,
  app and playtest.
- So is the never-emitted `death_star_shield_status` telemetry, also
  dropped from `scripts/eval_parity.py`.
- port: `FUN_005ad930`'s further test, more launched fighters on one side
  than the other, is not ported.

## Save v96

- `SaveState` loses `death_star`, so `SAVE_VERSION` 95 -> 96.
- The seed-42 golden is regenerated for this cause:
  - initial `v1:803494e978e8f71f` -> `v1:82332ef108ec7e21`;
  - final `v1:5eb68d3167d6a252` -> `v1:49a3738dca7ca496`;
  - two recordings are identical, and only the fingerprints changed.

## Tests

- `tactical_view.rs`:
  `a_death_star_shield_on_the_battle_system_bars_the_fighters_attack`. It
  covers no shield, a shield, a missing fighter selection, and one's own
  Death Star. It fails without the change (`fail-without.txt`).
- `combat.rs`:
  - `a_death_star_takes_hull_damage_in_an_auto_resolved_battle` replaces
    the three tests of the unsourced immunity;
  - its class is now CAPSHPSD 136, as seeding stores it.
- `cargo test --workspace`: 2487 passed, 0 failed.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all 24
are identical to the stage before. The immunity never applied to a
shipped class.

## Gates

A first run (`f9`) left three UI mutants alive, and its performance step
failed to start the browser (JSON decode error, no requests). The code was
fixed, and everything below is from the full rerun (`f10`).

- The replay equivalence check passes on the new golden with 0 browser
  errors (release WASM SHA-256
  `d7ee8e908a6ff87110dfde23c2d3d0a4ec08f639925b64c76fe56b5f363d46b3`).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.8 ms | 23.9 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.9 ms |

- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors; 4 requests, all HTTP 200.
- `cargo mutants --in-diff`: 20 mutants, 13 caught, 6 unviable, 1 missed.
  - The missed one is `has_selection`'s `||` in `draw_tactical_view`.
  - That line is unchanged context next to the edit, not this stage's
    code.
- `provenance-scan check`: 53 uncited items, unchanged.
- No browser pass of the shielded Attack Death Star was made.

Hashes of the committed files are in `SHA256SUMS`.
