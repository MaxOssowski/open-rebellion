# A manufactured ship travels to its destination, in the browser (F-030)

Date: 2026-10-20 (overnight run, "Produkcja i naprawy").

## The original

- A product bound elsewhere departs when its last point is done:
  `FUN_0052b960` -> `FUN_0052bee0` sets it EnrouteActive.
- It travels its own transit time (`FUN_0055d8c0`) and arrives on event
  `0x387` (`ghidra/notes/build-delivery.md`).
- F-030's port (2026-09-28, `delivery.rs`) still had no browser pass.

## The gate

- A new fixture scenario, `ProductionDelivery` (74, code 75), is
  `ProductionDestination` with three changes:
  - the primary shipyard's queue holds one capital ship of the player's
    side, one point from its cost;
  - the ship is bound for the loading target, another of the player's
    systems in the same sector;
  - the clock runs Fast.
- The observation adds:
  - the player's departure line ("Construction complete at X, en route
    to Y", notification `0x21`);
  - the player's arrival line ("Unit arrived at Y", `0xd`);
  - the player's capital ships at the target.
- Messages are filtered to the player's audience; a first run without the
  filter had matched an AI delivery's arrival.
- `fleet-window.mjs` `delivery` checks four things, for both sides:
  - before: nothing has departed and no ship is at the target;
  - departed: "Construction complete at Akrit'tar, en route to F'tral",
    and still no ship at F'tral, so it is en route;
  - arrived: "Unit arrived at F'tral", with one ship there;
  - 0 page errors, and 4 requests, all HTTP 200.
- The run used Chromium 154 through the Linux manifest copy.
- The screenshots were inspected: F'tral shows the fleet icon after the
  arrival. Their hashes are in `screenshots.sha256`; the checks are in
  `delivery-browser.json`.

## Gates (`f17`, shared with the KDY removal)

- The Fleet window suite passes all 52 cases. The Status window gate
  passes.
- The app's fixture tests pass, including
  `the_delivery_scenario_queues_a_ship_one_point_short_bound_for_the_target`.

Hashes of the committed files are in `SHA256SUMS`.
