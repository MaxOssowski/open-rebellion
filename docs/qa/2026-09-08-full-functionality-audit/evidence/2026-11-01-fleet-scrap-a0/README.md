# Scrapping a fleet (2026-11-01)

## A0

See `a0-experiment.md`: the player's own fleet shows Scrap enabled; the
confirmation lists the fleet's name; the fleet goes with everything
aboard, each ship and regiment refunding half its refined cost (20 for
Fleet 1, and 47 maintenance freed). An arriving fleet's Scrap is grey.

## Change

- `scrap.rs`: `ScrapTarget::Fleet`, its refund (half of every ship's,
  squadron's and cargo regiment's refined cost) and removal (cargo
  destroyed, characters adrift at the system, special forces landed);
  `check_with_movement` refuses a fleet in hyperspace (status 4).
  `scrap` and `scrap_team` take the troop transport state.
- `object_menu.rs`: a fleet maps to `ScrapTarget::Fleet`.
- `main.rs`: the menu gate and the menu command use
  `check_with_movement`; the checkmark validates again before scrapping
  (`FUN_0041ce20`).

## Result (Linux Chromium)

`fleet-window.mjs` `fleet-scrap` passes for both sides
(`gate-fleet-window-fleet-scrap.txt`; screenshots in `port-sha256.txt`):
a regiment boards the player's fleet; the fleet's menu shows Scrap
enabled; the confirmation reads "Are you sure you want to scrap the
following units? / Fleet 1"; the checkmark logs the fleet scrapped. The
Alliance run's top bar went from 0 to 49 refined and 678 to 751 free
maintenance, the same readouts A0 moved. Before the change the fleet's
Scrap was grey (the `rename` case's 2026-11-01 05:35 fleet menu), so the
case's `scrap_enabled` assertion failed.

Tests: `a_fleet_is_scrapped_with_everything_aboard_each_refunding_half`
(the A0 numbers, plus two squadrons, a character adrift and one on a
mission, a special force landed) and `a_fleet_in_hyperspace_refuses_scrap`;
`a_scrap_row_follows_its_gate_for_the_units_it_names` now lists the fleet.

## Mutants

`scrap.rs --re 'fleet_refund|scrap_fleet|check_with_movement|target_name|resolve|fn scrap|code'`:
37 tested, 34 caught, 1 unviable, 2 missed (a squadron count of 1 and the
on-mission character); after the test took two squadrons and a character
on a mission, `--re 'fleet_refund|scrap_fleet'`: 18 tested, 18 caught.
