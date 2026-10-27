# The Battle Alert lists each ship's squadrons under it (P24)

Date: 2026-10-27. The manual's Fig. 4.2 lists "the capital ships ... and
their assigned fighters, troops and personnel". The port's Battle Alert
listed all of a fleet's squadrons after all its ships. Since save v106
(`../2026-10-26-squadron-carriers/`) each squadron knows its ship, so
they can go under it.

Source: `ghidra/notes/battle-alert-window.md`, "The list tabs", traced
2026-10-22.

## The original

`FUN_00450770`, a side's forces tab. For each fleet the side has at the
system (`FUN_004ffef0`):
1. the fleet's row;
2. each capital ship (`FUN_00502db0`, font 5, with its mini);
3. under each ship and indented (`FUN_00450ce0` with 1): the ship's own
   squadrons (`FUN_005039d0`), then its regiments (`FUN_00504c40`), then
   its personnel (`FUN_00536da0`).

## The port

`battle_alert_rows` in `crates/rebellion-render/src/tactical_view.rs`:

- each ship's row is followed by the squadrons whose `carrier` is that
  ship's number, indented;
- port: squadrons on no ship of the fleet follow the ships, then the
  fleet's regiments and personnel. The port keeps those on the fleet, not
  on a ship.

## Tests

- `the_battle_alert_lists_each_ships_squadrons_under_it`
- `the_battle_alert_lists_a_side_s_fleets_then_its_planet_squadrons`
  (unchanged: its ship has no number, so its squadron follows the ships)

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2965 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | first run 9 tested: 2 missed. After a squadron whose ship is gone was added to the test: 9 tested, 8 caught, 1 unviable |
| Release WASM SHA-256 | `ff943060677577c77bc3eddcf3347960fba141a69075a6f55f98af2d3989ae85` |
