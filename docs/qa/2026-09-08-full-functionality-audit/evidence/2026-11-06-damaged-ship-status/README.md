# A damaged ship's status against A0 (2026-11-06)

This covers OBJ-06-C002 ("damaged").

## A0

The A0 run is the Alliance game after the battle at Umgul; the captures are in `.artifacts/a0-vm/fleet2/`.

- **`05-medium-transport-1-status` (day 714).** Medium Transport 1 in Fleet 1:
  - Status Awaiting Orders, Maintenance Cost 12.
  - Capacity 0 squadrons and 2 regiments, with 2 regiments embarked.
  - Ship Damaged "Yes", Hyperdrive Rating "0:100" (both hyperdrives gone), Hull Value "317:360".
- **`06-corellian-corvette-1-status` (day 728).** Corellian Corvette 1 reads Ship Damaged "Yes", Hyperdrive Rating "80:80" and Hull Value "240:500": a hull hit alone.
- **`umgul-fleet1-damaged-*`.** The damaged fleet's marks in the Fleet window. `fleet-window.mjs` `a0-umgul` already compares them with A0 on Linux Chromium.

## The test

`a_damaged_medium_transports_status_reads_as_a0` (rebellion-app, run with `--ignored` on the original `data/base`) builds both ships from CAPSHPSD:
- a Medium Transport at hull 317, with one hit on each hyperdrive and two regiments aboard;
- a Corellian Corvette at hull 240.

It then reads `status_rows::ship_rows` (`FUN_00446fd0`). The rows equal A0's: the transport's fourteen, and the corvette's Ship Damaged, Hyperdrive Rating and Hull Value.

`an_undamaged_corellian_corvettes_status_reads_as_a0` settles OBJ-06-C001 the same way.
