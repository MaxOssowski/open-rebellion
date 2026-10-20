# A delivery to a destroyed system autoroutes instead of being lost (F-030, save v100)

Date: 2026-10-20 (overnight run, "Produkcja i naprawy").

## The original

- An object en route is already a child of its destination.
- When that system is destroyed, its destroyed hook (`FUN_00510950` ->
  `FUN_004fb520`) calls slot `+0xb4` on each child. `FUN_004f8710`
  destroys only a completed child that is not EnrouteActive. A travelling
  product is set Autorouting instead, and the autoroute search
  (`FUN_005545e0`) sends it to the nearest place of its side.
- `GameObjDestroyedOnArrivalNotif` (`0x303`, `FUN_004fc080`) reports an
  object destroyed while en route; it does not destroy one.
- `0x321` is the manager's current-product notification
  (`ManuMgrDeploymentKeyNotif`), not a redirect. The Destination command
  rewrites only queued products, so there is no redirect of an object
  already en route.
- The notes are in `build-delivery.md`, "A destroyed destination".

## What the port did before

`DeliveryState::advance` lost every delivery whose destination had been
destroyed, and reported it as "destroyed on arrival".

## Port

- `Delivery::side`: the building system's holder at departure (save
  v100).
- An arrival at a destroyed system sets out again for
  `missions::nearest_place` of that side. That is the same candidate
  order the port uses for autorouting characters, now `pub(crate)`. The
  new trip is reported in `DeliveryAdvance::rerouted`.
- Only a delivery with nowhere to go is lost.
- port: the new trip starts on the arrival day from the destroyed system,
  not on the day of the destruction.

## Tests

- New: `a_delivery_to_a_destroyed_system_reroutes_to_the_nearest_place_of_its_side`.
  It fails when the reroute is disabled (`fail-without.txt`).
- Renamed: `a_delivery_to_a_destroyed_system_with_nowhere_to_go_is_lost`.
- `cargo test --workspace`: 2489 passed, 0 failed.

## Save v100

- `Delivery` gains its side, so `SAVE_VERSION` 99 -> 100.
- The seed-42 golden is regenerated for this cause:
  - initial `v1:fe91e55dd0f9a2af` -> `v1:38af87970fe0c218`;
  - final `v1:b51db94b116e8495` -> `v1:eea438fc7e6c6fbc`;
  - two recordings are identical, and only the fingerprints changed.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:
- 23 are identical to the stage before;
- seed 10 changes on day 1388:
  - an Alliance production facility built at Vagran for Ghorman, which
    had been destroyed, was "destroyed on arrival";
  - it now travels on to the nearest Alliance system.

## Gates (`f18`)

- The replay equivalence check passes on the new golden with 0 browser
  errors (release WASM SHA-256
  `1462f4477493e4f15477344c9df9cb66c5973f28265a7223ffb87e6f229d9a92`).
- The Fleet window suite passes 52 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- `cargo mutants --in-diff`: 7 mutants, 7 caught.
- `provenance-scan check`: 51 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
