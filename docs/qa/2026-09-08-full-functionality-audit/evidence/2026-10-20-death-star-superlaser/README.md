# The strategic superlaser and the nearby warning had no source (F-017, save v95)

Date: 2026-10-20. Stage DS4 of the "Gwiazda Śmierci" area, on the user's
go-ahead.

## The original

- A planet falls only to the Fleet menu's Destroy System order (`0x223`).
  `FUN_00582440` enables it for a fleet holding a Death Star, and
  `FUN_0058d990` carries it out. Both are already ported
  (`bombardment.rs`, `2026-10-20-death-star-destruction`).
- The F-017 search found no other planet-destruction notifier.
- In a tactical battle the superlaser charges for 300 s and destroys a
  chosen capital ship (`tactical-death-star-path.md`). That is already
  ported in the tactical session.
- DeathStarNearby (system `+0x88` bit 11) is set for every system of a
  sector that holds an active Death Star (`FUN_005178b0`). It is already
  ported as `economy::death_star_nearby`.

## What the port did before

The strategic Death Star state kept a fleet pointer,
`DeathStarState::death_star_fleet`, that nothing in the game ever set.
Everything built on it was unreachable and had no source:
- `DeathStarSystem::fire()` and its preconditions;
- the panel's Fire and Move buttons (`PanelAction::FireDeathStar`,
  `MoveDeathStar`);
- `DeathStarSystem::advance()`'s `NearbyWarning`, within an invented
  300-unit radius, with its message and advisor line;
- `tactical_flow::reconcile_death_star_result`.

The panel (key D) therefore always read "No Death Star in service".

## Port

- All of the above is removed.
- `DeathStarEvent` keeps only `PlanetDestroyed`, which the Destroy System
  order carries.
- `DeathStarState` keeps only the battle shield flag.
- The port-only panel (D) is now read-only: it lists the side's fleets
  that hold a Death Star (`panels::death_star::death_star_fleets`), with
  their location and ship count. It points to the Fleet menu for Destroy
  System.
- The integrator's Death Star test checks that a completed Death Star's
  fleet may order Destroy System (`bombardment::order_enabled`).
- The provenance baseline drops `DeathStarSystem::advance`, which goes
  from 54 to 53 uncited items.

## Save v95

- `DeathStarState` loses `death_star_fleet`, so the bincode layout
  changes: `SAVE_VERSION` 94 -> 95.
- The seed-42 golden is regenerated for this cause:
  - initial `v1:f47443f73b78e129` -> `v1:803494e978e8f71f`;
  - final `v1:caadbc1bab82cbda` -> `v1:5eb68d3167d6a252`;
  - two recordings are identical, and only the fingerprints changed.

## Tests

- Eight tests of the removed, unreachable path go with it: five of
  `fire()`, two of the warning, one of the reconcile.
- New: `the_panel_lists_only_the_side_s_fleets_holding_a_death_star`.
- `cargo test --workspace`: 2489 passed, 0 failed.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all 24
are identical to the stage before.

## Gates

A first gate run was started twice by mistake, and the two runs wrote over
each other's logs. Everything below is from the clean third run (`f8`).

- The replay equivalence check passes on the new golden with 0 browser
  errors (release WASM SHA-256
  `7902fc785e44e9192e7d0041aa737f301070f8b4f9ebc2e25f875b2152624a7f`).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.8 ms | 23.5 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.7 ms |

- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors; 4 requests, all HTTP 200.
- `cargo mutants --in-diff`: 6 mutants, 5 caught, 1 missed.
  - The missed one replaces the panel's egui drawing (`draw_death_star`)
    with `()`. The drawing has no test harness; its data,
    `death_star_fleets`, is tested.
  - A first run also missed `draw_death_star`'s always-`None` return and
    its side test. The return type is now `()`, and the side test moved
    into `death_star_fleets`.
- `provenance-scan check`: 53 uncited items (one cleared).
- The read-only panel had no separate browser pass.

Hashes of the committed files are in `SHA256SUMS`.
