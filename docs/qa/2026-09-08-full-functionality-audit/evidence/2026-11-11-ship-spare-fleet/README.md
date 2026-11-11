# A built ship's fleet and the ship band

Date: 2026-11-11. Cell OBJ-06-C011 (a ship order's Confirm and the band
it fills). The trace is `ghidra/notes/ship-deployment-fleet.md`.

## A0

Windows 10 VM, original REBEXE, the Empire game of
`2026-11-11-icon-menu-and-topic-box-a0`. The captures are in
`.artifacts/a0-vm/2026-11-11-icon-menu-topic/`, with their SHA-256 in
`SHA256SUMS`.

| Capture | Seen |
| --- | --- |
| `60`, `61` | two Orbital Shipyards ordered at Coruscant; the facility band reads "Orbital Shipyard / Building: 2 / Destination: Coruscant" |
| `62` | Build Selection on the ship band before any maintenance was free: n/a, Confirm disabled |
| `63`, `64` | Carrack Light Cruiser: 26, 24, "52 Days" for one; 104, 96, "208 Days" for four |
| `65` | five: n/a (the maintenance check) |
| `66` | four confirmed: "Carrack Light Cruiser 2 / Building: 4 / Destination: Fleet" |
| `67`, `68` | the Carracks in a new Fleet 4 beside Fleet 1; Create Fleet makes Fleet 5 |

Later the band read "Carrack Light Cruiser 4 / Building: 2" and
"Carrack Light Cruiser 5 / Building: 1" (not kept), then, for a new
single order, "Destination: Fleet 4".

## The original

A ship order creates its products at Confirm in the destination
system's bit-1 spare fleet (`FUN_005385f0` → `FUN_00552300` →
`FUN_005097d0`; `FUN_0052a140`). The band shows the manager's
DeploymentKey (`+0x78`, `FUN_004591b0`), which is the current product's
container (`FUN_0052ac40`):
- "Fleet" while the spare is unnumbered;
- "Fleet N" once its first ship completes.

The filled spare stays the spare (`FUN_0050bc60` clears bit 1 only for a
destroyed fleet) until it leaves the system (`FUN_004ff570`). The
products are named objects from the order, "{class} {N}"
(`FUN_00517760`).

## The port before

- A completed ship joined the first fleet of its side in the system
  (`apply_build_completion_inner`), here Fleet 1.
- The band read "Destination: Coruscant" and "Carrack Light Cruiser".

## The change

- `GameWorld::ship_spare`, `set_ship_spare` and `forget_ship_spare`, with
  `FleetNames::ship_spares` (save v114, `agent_docs/save-load.md`).
  `ship_spare` answers only while the fleet exists, is of the side, and is
  in the system's fleet list at that system.
- `apply_build_completion_inner` (`rebellion-data`) puts a completed
  capital ship in the system's spare. With none, it makes a new fleet
  (numbered by `insert_fleet`) and makes that fleet the spare.
- `begin_fleet_transit` forgets a fleet's spare role when it leaves.
- `manufacturing_window::band`, for a capital-ship product:
  - "Destination: " + the destination system's spare name, else "Fleet";
  - the product as "{class} {next number}"
    (`GameWorld::next_ship_number`), not for the Death Star.

port: the original's spare exists before any ship, and the product sits
in it from the order. The port makes the fleet with the first completed
ship and numbers a ship as it completes. So two orders of one class in
two systems may take their numbers in another order.

hyp, not traced: fighters, regiments and facilities keep naming the
system. A Destination order to another system routes the ships into that
system's spare, which the band names from the destination system.

## Tests

- `integrator::tests::a_completed_ship_starts_the_systems_spare_beside_its_fleet_and_the_next_joins_it`.
  It fails with the old rule: panicked at the standing fleet's count.
- `manufacturing_window::tests::a_ship_band_names_the_spare_and_the_next_numbered_ship`.
- Changed on this A0: `a_building_band_shows_its_product_units_progress_and_destination`
  (now "Corellian Corvette 1", "Destination: Fleet") and
  `system_window::tests::a_building_band_shows_its_product_and_units`
  ("Corellian Corvette 1").
- `world::ship_spare_tests` (four tests: per system and side, the spare's
  own checks, a forgotten spare, the next ship number).

Workspace tests in a separate worktree: 3566 passed, 0 failed; the seed 42
replay golden is unchanged.

Mutants (in place, the diff, with the core, data and render tests): the
first run missed 10 in `GameWorld::ship_spare`, `set_ship_spare`,
`forget_ship_spare` and `next_ship_number`; `ship_spare_tests` closes
them (23 of 23 caught in `world/mod.rs`). The other 18 of the first run
were caught, 1 unviable.

Save format v113 -> v114 (`FleetNames::ship_spares`).

## Batch b188

- 29 of 29 interface gates on Linux (`cutscenes`
  run `cutscenes-2026-11-11T11-51-49-653Z`).
- Workspace tests: 3570 passed, 0 failed. Fixture tests: 150. Provenance
  is ok.
- Tactical matrix `2026-11-11T12-01-02-268Z-460396`: 148 of 148.

Not yet covered at b188: no browser gate drove a ship order to Confirm
and read the band against A0 `66`. The next section closes that.

## The browser gate

`build-selection.mjs` has a new case, `shipOrderBandCase`, run on both
sides after the fleet destination case. On fixture 89 it opens Build on
the ships band and confirms an order:
- the Empire: four Carrack Light Cruisers, as in A0 `64` and `66`;
- the Alliance (no A0 here): one of its first class.

The fixture's production report now carries each band's three drawn
lines (`FixtureBand::lines`, from `manufacturing_window::band`). The case
checks:
- the band holds the order;
- its first line is "{class} {N}";
- its second line is "Building: {count}";
- its third line is "Destination: Fleet".

The number is not compared, since it depends on the side's earlier ships
of the class: A0's Empire had one Carrack, and the fixture's has none.

Run on Arch Chromium (the Linux manifest):
`.artifacts/interface-parity/build-selection-2026-11-11T12-18-07-434Z-495278`,
pass. The bands read:
- the Empire: "Carrack Light Cruiser 1 / Building: 4 / Destination: Fleet";
- the Alliance: "Alliance Dreadnaught 1 / Building: 1 / Destination: Fleet".

`empire/ship-order-band.png` was compared by eye with A0 `66`: the same
three lines, in the same places on the band.

## Batch b189

- 29 of 29 interface gates on Linux, among them `build-selection`
  (`build-selection-2026-11-11T12-39-42-512Z-572262`) with the new
  case.
- Workspace tests: 3572 passed, 0 failed. Fixture tests: 151. Provenance
  is ok.
- Tactical matrix `2026-11-11T13-23-46-184Z-633999`: 148 of 148.
