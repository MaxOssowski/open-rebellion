# Menus of a fleet in hyperspace against A0 (2026-11-01)

## A0 (VM Alliance game, days 318-330)

Fleet 4 at Kimanan, status "Enroute", ETA day 356
(`menus/32-fleet4-status.png`):

- its menu (`22-fleet4-menu.png`, `31-fleet4-menu-day327.png`) greys
  every row but Encyclopedia and Status: Move, Confirmed Move, Planetary
  Bombardment, Planetary Assault, Rename and Scrap;
- its ship, Corellian Corvette 2 (`29-own-ship-menu.png`,
  `30-own-ship-menu.png`), likewise: Move, Confirmed Move, Create Fleet,
  Rename and Scrap grey.

Hashes in `a0-sha256.txt`. `FUN_004f9860`, the object check behind the
Scrap command's `+0x5c` (`scrap-order.md`), refuses an object en route
with status 4.

## Change

- `scrap::check_with_movement` also refuses a ship whose fleet is in
  hyperspace (`FUN_005575f0` → `FUN_004f9860`).
- `order_gates` (`main.rs`): Rename on a fleet or ship is off while the
  fleet is in hyperspace. hyp: Rename's `+0x18` runs the same object
  check; the captures show it grey.

The port already greyed Move, Confirmed Move, Create Fleet and the
bombardment targets for a fleet in transit.

## Tests

- `a_fleet_in_hyperspace_takes_neither_rename_nor_scrap` (`main.rs`):
  the fleet's and its ship's Rename and Scrap are on in orbit and off in
  hyperspace. Without the Rename condition it fails on the fleet.
- `a_fleet_in_hyperspace_refuses_scrap` (`scrap.rs`) now covers the ship;
  without the ship arm it fails (`Ok(())` for `Err(EnRoute)`).
- `fleet-move.mjs` `move` checks the fleet's Rename and Scrap are on
  before it leaves.

## Browser (Linux Chromium 155)

`fleet-move.mjs en-route-menu`, both sides: Move sends the fleet to the
target, a double click on the target planet's fleet icon opens its Fleet
window, which lists the fleet en route (`listed_fleets`, as A0 lists
Fleet 4 at Kimanan, `18-fleet4.png`), and its right-click menu greys
Move, Confirmed Move, Planetary Bombardment, Planetary Assault, Rename
and Scrap, leaving Encyclopedia and Status, row for row as
`22-fleet4-menu.png`. The fixture reports the fleet's entry in the
destination window (`destination_entry`, from `FleetWindowReport`).

Run `fleet-move-2026-11-01T08-46-29-577Z-7638` (Arch Chromium
155.0.8059.84, not the pinned build), test WASM
`6dee2dba830f5d4ccae135f33ffe763098c9549986b49347d642ff683e5777f3`:

```
03af061603da473fa1adc9dfdf52b226b421252699446abfc1c96b7181bc2381  alliance-en-route-menu/en-route-menu.png
c89de6cb0d9bd580b2ab1bd115363bbb8239786851c86ea5744313a205780625  empire-en-route-menu/en-route-menu.png
```

The earlier note that a fleet in transit is in no Fleet window was
wrong: the port lists it at its destination since `8fc9d983`
(`a_fleet_in_hyperspace_shows_its_overlays_where_it_is_bound`); the gate
only had no case that opened that window. The case waits 0.6 s before
the double click, or the target's click and the icon's first make a
triple click in egui.

## Mutants

- `scrap.rs --re check_with_movement`: 3 tested, 3 caught.
- `main.rs`, the Rename gate's lines (9255-9276): 9 caught. The test also
  checks the other side's fleet and ship take neither Rename nor Scrap,
  which kills the ship gate's `is_alliance && alive` → `||`. Unrelated
  misses reported in the same run (`window_conf` fields,
  `draw_fullscreen_texture`'s `dest_size`) lie outside this change.
