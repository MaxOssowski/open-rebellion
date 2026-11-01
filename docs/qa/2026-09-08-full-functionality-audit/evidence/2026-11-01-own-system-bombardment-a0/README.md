# Bombardment over the player's own system (2026-11-01)

## A0 (VM Alliance game, day 316)

- `menus/12-own-fleet-menu.png`: the player's Fleet 1 at Kimanan, an
  Alliance system, in its Fleet window. Move, Confirmed Move, Planetary
  Bombardment, Rename, Encyclopedia, Status and Scrap are white; Planetary
  Assault is grey.
- `menus/13-own-bombard-sub.png`: the submenu lists Target Military
  Facilities, Target Civilian Facilities and General Bombardment white,
  Destroy System grey (no Death Star).
- `menus/15-own-bombard-result.png`: choosing Target Military Facilities
  carries the strike out: "Orbital bombardment of Kimanan", "Alliance
  ships have conducted an orbital strike on the Alliance system of
  Kimanan."

Hashes in `a0-sha256.txt`. The order's enabled check `FUN_00581e80`
(`FUN_00553770`, `FUN_00582440`) reads no side.

## Change

`bombardment::order_enabled` no longer refuses the side's own system.
That condition was a port rule taken from the manual ("in orbit around
an enemy or neutral system"); A0 contradicts it. The fleets must still be
the side's, in orbit and out of hyperspace, with bombardment, and the
system must stand.

`a_bombardment_needs_the_side_s_fleets_in_orbit_with_bombardment` (AI
validation) expected the own system refused; it now expects it accepted,
citing the capture. With the old rule that assertion fails.

## The menu against A0

The port's own-fleet menu (`fleet-window.mjs` rename case, Alliance,
`port-sha256.txt`) now matches `12-own-fleet-menu.png` row for row except
Scrap, which the port still greys: scrapping a fleet is not ported
(`scrap-order.md`, Open). OBJ-14-C002 stays partial.

## Mutants

`bombardment.rs --re order_enabled`: 20 tested, 20 caught.
