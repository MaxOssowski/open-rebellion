# Build Selection's number, Cancel, and a band's selection after its menu (2026-11-02)

Surface ledger:

- OBJ-08 (Build Selection): C009 "quantity" and C012 "cancel".
- OBJ-07 (Manufacturing): C007 "queue populated".

## A0

The captures are in `.artifacts/a0-vm/build-selection2/`, with hashes in `a0-sha256.txt` (see `2026-11-02-production-stop-confirmation`). They come from the Empire's "resource turn 14" save in the Windows 10 VM. Coruscant's window is at (203, 91) and Build Selection at (255, 87).

- **`01`..`04`.** The Imperial Army Regiment at Coruscant, with one training facility.

  | Number | Refined | Maintenance | Completion | Deployment |
  |---|---|---|---|---|
  | 1 | 6 | 3 | 24 Days | 0 Days |
  | 3 | 18 | 9 | 72 Days | 0 Days |
  | 2 | 12 | 6 | 48 Days | 0 Days |

  The costs and the single yard's completion scale with the number. The deployment does not.
- **`05`, `06`.** Cancel closes the window and the band is unchanged. Build reopens the window at 1.
- **`07`.** After Confirm, the band shows:
  - the name and "Training: 1";
  - the mini GOKRES 1154 at the window's (95, 153), with its 96% non-key pixels found. The port puts it at the band's (40, 15), which is the same place.
  - an unselected title strip.
- **`16`..`18`, the band's selection** (strip colour at x 380):

  | Capture | Ships | Troops | Facilities |
  |---|---|---|---|
  | `16` after a left press on troops | (91, 179, 75) | (0, 255, 0) | (91, 179, 75) |
  | `17` with the facilities band's menu open | (91, 179, 75) | (91, 179, 75) | (0, 255, 0) |
  | `18` after Escape | (91, 179, 75) | (91, 179, 75) | (91, 179, 75) |

  A left press keeps its band selected. A band's menu selects its band only while it is open.

## The port

- **Deselecting.** `SystemWindowState::deselect_band` runs in `main.rs` when an object menu opened for a `MenuObject::Producer` closes, whichever row was chosen or none. Before this change, the band kept its lit strip after its menu.
  - hyp: the right press selects only for the menu. The end of `FUN_004ac5c0` is untraced.
- **Fixture report.** The production report now carries Build Selection's number and its four values as drawn: `build_selection_quantity` and `build_selection_values`.
- **`build-selection.mjs`, on both sides:**
  - presses the up arrow twice: number 3, the costs and completion tripled, the deployment unchanged;
  - presses the down arrow once: number 2;
  - presses Cancel: the window closes and the band stays empty;
  - reopens with Build: number 1.
  - For the Empire, it also compares 21 by 10 text-free pixels of the confirmed band's title strip with A0 `07`. They must be equal.
- The fixture builds another class, the Stormtrooper Regiment, so the values are checked for their scaling, not against A0's numbers.

## Tests

- `a_band_menu_closing_leaves_no_band_selected_in_that_window_only` (`system_window.rs`).
- `the_menu_reports_each_rows_label_and_state_in_order` now also checks `ObjectMenuState::selection` (`object_menu.rs`).

## The idle overview (OBJ-07 C006, C008)

A0 `11-stopped` shows Coruscant with every band idle. `build-selection.mjs` compares the Empire's idle overview with it (`idleBandsAgainstA0`).

Each band's status line (TEXTSTRA 6198..6200 at the band's (5, 16)) and the troops' and facilities' "1:1" counts (`FUN_00457690`) are compared by their ink boxes relative to the window. Run `build-selection-2026-11-02T08-35-16-181Z-3818211` (Linux Chromium 155, test WASM `354677a3…`):

| Text | A0 | Port |
|---|---|---|
| "No Ships are being built" | 60, 73..204, 84 | 61, 72..204, 84 |
| "No Troops in training" | 60, 157..157, 165 | 60, 157..158, 165 |
| "No Facilities are being built" | 60, 238..185, 246 | 61, 238..186, 246 |
| troops "1:1" | 21, 203..34, 212 | 21, 204..37, 212 |
| facilities "1:1" | 21, 283..34, 292 | 21, 284..37, 292 |

- The counts' 3 extra pixels are Liberation Sans's footed "1" against GDI Arial's plain one. This is the accepted text deviation.
- The band titles are black in both: A0 draws them crisp, the port antialiased.
- A0's Coruscant has no shipyard ("0:0"), so the ships' count is not compared.

## How a band's menu closes (corrected the same day)

`20`..`24` refine the rule above. They sample the strip colour at x 380:

| Capture | Ships | Troops | Facilities |
|---|---|---|---|
| `21`, after the ships band's Destination released on Chandrila | lit | | |
| `22`, the facilities band's menu | | | lit |
| `23`, that menu dismissed by a click beside it | | | lit |

Only the ships band changes its destination ("Destination: Chandrila"). The others keep Coruscant, which shows Destination stored per producer.

So the selection ends on:

- Escape (`18`);
- Build and Stop, whose windows open over it (`05`, `07`, `11`, `15`).

It stays after Destination and after a click beside the menu.

- `system_window::band_menu_deselects(command, escaped)` holds this rule. `main.rs` applies it when a band's menu closes.
- hyp: the windows' activation clears the selection. The other rows are not captured and keep it.
- Test: `escape_build_and_stop_end_a_bands_selection_but_destination_and_a_click_beside_keep_it`.

## A band's Destination in the browser (OBJ-07 C015, C016)

Scenario `BuildSelectionDestination` (88, code 89) opens the secondary system's sector with the Manufacturing window beside it at (360, 76). It reports each band's Destination system.

`build-selection.mjs` releases the troops band's Destination on the secondary's planet. On both sides the troops band then names it ("F'tral") and the ships and facilities bands keep their own system ("Akrit'tar"). This is A0 `21`'s per-band rule.

Run `build-selection-2026-11-02T10-…` (Linux Chromium 155, test WASM `07d33e9a…`).

- The batch's other gates and checks passed on the run before this scenario moved its window aside.
- That first run's build-selection failed because the target planet lay under the window.

## Destination released on windows (OBJ-07 C017..C019)

A0 `25`..`35` (the ships band of Coruscant):

| Release | Result |
|---|---|
| On Coruscant's Defenses window (`25`) | "Destination: Coruscant": accepted |
| On Coruscant's own Manufacturing window, from Chandrila (`26`, `27`) | Coruscant: accepted |
| On Yavin's planet, an enemy system (`33`) | "Destination: Yavin": accepted |
| On the Fleet window of Imperial Fleet 1 at Yavin (`30`, `32`, twice) | unchanged |
| On Selonia's Fleet window, an own system (`35`) | unchanged |

- **Fleet windows: open.** A0 left the destination unchanged on every release on a Fleet window. Two explanations remain:
  - the release is refused there;
  - the click only activated that window.

  The port still accepts a Fleet window release (`ReleaseTarget::Fleet`).
- **The Manufacturing window.** `build-selection.mjs` now also releases the troops band's Destination on the Manufacturing window itself. The destination returns to the window's system, as A0 `27` shows. Run `g-build-selection-b21`: pass on both sides.
- `28-enemy-yavin-manufacturing` is the enemy Yavin's Manufacturing window, opened by double-clicking its facility icon. Its strip is red.

## A0's class and values (OBJ-07 C020, OBJ-08 C003)

The fixture report now also names the shown class (`build_selection_class`).

For the Empire, `build-selection.mjs` steps the class list to the Imperial Army Regiment. The open list starts at the shown class, so the gate picks its second item each time. The values must be A0 `01`'s exactly: 6, 3, "24 Days", "0 Days". Coruscant and the fixture's system both have one training facility.

Run `build-selection-2026-11-02T11-34-51-460Z-4170502` (Linux Chromium 155, test WASM `19679add…`): equal.

A0 shows the best completion as a length in days ("24 Days"), not an absolute game day, and so does the port. The cell's wording, "an absolute game day", does not match A0.

## Destination released on a Defenses window, in the browser (OBJ-07 C017, 2026-11-03)

**A0.** `build-selection2/25`: the ships band's Destination, released on Coruscant's Defenses window, makes that system the Destination.

**The port.** The new fixture `BuildSelectionDefensesDestination` (code 95) opens the Manufacturing window at (360, 76) and the secondary system's Defenses window at the galaxy view's left.

The build-selection gate's `defensesDestination` step does the following on both sides:

- right-clicks the ships band;
- chooses Destination;
- clicks inside the Defenses window;
- waits for the band to read the window's system.

Run `build-selection-2026-11-03T04-16-07-647Z-2127650` (Linux): the Alliance's Destination goes from Akrit'tar to F'tral. Screenshot: `defenses-destination.png`.

## No room aboard, Stop, and the other side's fleet (OBJ-07 C019, 2026-11-21)

These settle the three deviations left below. The traces are in
`ghidra/notes/production-destination.md`: "Build with no room aboard",
"Stop resets the Destination" and "The other side's fleet".

- **No room aboard (ported).** Build Selection's check (`FUN_005385f0`)
  runs the planner per unit on the manager's TargetKey (`FUN_00551a20` ->
  `FUN_00552300`). A fleet weighs its ships; a capital ship is weighed
  alone. A ship with no troop or fighter room left records `{1, 0x27}`
  (`FUN_00550700`), and the order is not valid. Build Selection then shows
  "n/a" and greys Confirm (`FUN_00439160`). The units the order replaces
  give their seats back. The port's check is `build_selection::lacks_fleet_room`,
  part of `estimate`.
- **Stop (ported).** Stop scraps the queue (`FUN_00529ba0` ->
  `FUN_0052a2c0`). The scrapped products' `0x300` events bring
  RemainingGameObjCount to 0, and `ManuMgr_vt204` (`FUN_0052b8c0`) sets the
  TargetKey to none, so the band names its own system again. Build scraps
  too, but creates its units first and keeps the Destination (A0 12).
- **The other side's fleet (not found).** The checks read so far
  (`FUN_00537180`'s `0x28`, its per-command checks, the route
  `FUN_005513a0`, `Fleet_vt74` -> `FUN_00553410`, `ManuMgr_vt1e4`) do not
  refuse the empty ships band of A0 `build-selection2/30`, `32`. The port
  keeps its refusal (`reason=other_side`).

A0 shows neither a Build refused for room nor a Stop on a band with a
Destination elsewhere. Both ports rest on the decompile.

**The gate.** The `fleetDrag` step now goes on after the regiment
confirmed for the ship:

- it opens Build Selection on the troops band again and raises the
  number until the fixture reports the fleet-room refusal; at that number
  the four values read "n/a" and Confirm is disabled, while the number
  before shows values;
- it stops the troops band, and the band names its own system again.

Run `build-selection-2026-11-21T05-45-50-611Z-978556` passed on both
sides: Linux, Arch Chromium 157.0.8103.78 through a temporary
`.gate-build-selection-linux.mjs` copy, cargo-built test WASM without
wasm-opt (sha256 `7a85fbfb798b8ce6…`), `result.json` sha256
`501d2644fd983872…`. On both sides Imperial Star Destroyer 1 seats 3
regiments: 3 show values and 4 read "n/a" (`fleet-drag-no-room.png`).
After Stop the troops band reads "No Troops in training" and
"Destination: Akrit'tar" (`fleet-drag-stopped.png`). The base build has
no fleet-room refusal, so the "n/a" assertion fails there; the unit tests
`a_build_for_a_fleet_or_ship_without_room_for_its_units_is_not_valid`,
`a_squadron_build_for_a_fleet_needs_fighter_room_aboard` and
`stopping_an_area_that_builds_resets_its_destination_but_a_new_order_keeps_it`
fail without the change.

## The A0 own-fleet gaps closed (OBJ-07 C019, 2026-11-21)

These close the rows left open in the table below.

- **The regiment's ship (A0 12, 13, 17).** `FUN_00552300` weighs the fleet,
  then each of its ships (`FUN_00552b10`). `FUN_0054fbb0` keeps a later
  ship only when it has strictly more room at record `+8`.
  `CapShip_vt78` (`FUN_00500b40`) gives that room: the troop capacity
  less the regiments aboard, or the fighter capacity less the squadrons.
  - With Fleet 1's Imperial Star Destroyer (3) and Victory Destroyer (2)
    and four regiments aboard, the regiments seat ISD, ISD, VD, ISD, so
    the fifth goes to the Victory Destroyer, as A0 shows.
  - The port seats the unit (`products::seat`). The band names that ship
    (`deployment_ship`), and the Units Deployed body names it too
    (`boarded_ship`, `travelling_ship`).
  - port: regiments ride a fleet as one list, so the carried regiments
    are seated again in order by the same rule.
- **The ship Destination (A0 34).** `FUN_00537180` takes a capital ship.
  A band released or dropped on a ship's entry of the Fleet window names
  that ship (`set_ship_destination`), and its units go to that ship
  while it has room.
- **The band drag (A0 28..33).** `FUN_004534f0` drags a band:
  - the drag image is STRATEGY 0x2841 for the Alliance and 0x2844 for
    the Empire, with hotspot (3, 3), keyed;
  - a release 4 or more pixels away gives a `0x214` against the `+0x70`
    object (`FUN_00422ce0`, window type 9).

**The gate.** Run `build-selection-2026-11-21T05-16-51-201Z-727725` passed
on both sides: Linux, Arch Chromium 157.0.8103.78 through a temporary
Linux copy, cargo-built test WASM sha256 `933753032cc073fa…`,
`result.json` sha256 `4ecfccf98103c4ee…`. Its `fleetDrag` step:

- drags the ships and troops bands onto Fleet 1, and both read
  "Destination: Fleet 1"; their lines match A0 33 and 31 (ships
  [6,61,93,66] against [5,60,92,66]);
- finds the drag image under the pointer (114 and 116 white pixels);
- drops the facilities band, which is refused (`0x25`);
- drops the troops band on the fleet panel's first ship, and it reads
  "Destination: Imperial Star Destroyer 1";
- confirms an Army Regiment there, and the band keeps naming the ship
  ("Training: 1"; deployment 17 days).

The message bodies and the per-ship seating are checked by unit tests,
not in the browser. A0's ship names differ from the fixture's, so those
lines are not compared by pixels.

**Remaining port deviations.** Settled in the section above: the Build
with no room aboard and the Stop are ported; the check that refuses the
other side's fleet is still not found.

## Against A0 on an own fleet (OBJ-07 C019, 2026-11-21)

**A0.** `.artifacts/a0-vm/2026-11-21-build-dest-16` is an unedited capture:
the Empire, slot 5, with its own Fleet 1 at Averam. Frame by frame:

| A0 frames | Original | Port |
|---|---|---|
| 04, 07, 31, 33 | the ships and troops bands take Fleet 1: "Destination: Fleet 1" | match; the gate compares both lines with 04 and 07 |
| 10, 29 | the facilities band is refused silently and stays "Destination: Coruscant" | match (`reason=0x25`, nothing shown) |
| 11, 22 | Best Time To Deployment counts the trip to the fleet's system | the port measures to the fleet's current system (`products::delivery_system`); the galaxy differs, so the days are not compared |
| 23, 24 | a Carrack ordered for Fleet 1 stands in Fleet 1 at once, unfinished; the band keeps "Destination: Fleet 1" | match in the unit tests (the product sits in the fleet; the band names the container); not in the browser |
| 25 | the Carrack finishes in Fleet 1 | match |
| 14, 25 | the band reads "Destination: Coruscant" again once its last unit completes | **fixed**: `ManuMgr_vt204` (`FUN_0052b8c0`) clears the TargetKey when the remaining count reaches 0 |
| 18 | a band that built nothing keeps Fleet 1 | match |
| 26, 27 | "Carrack Light Cruiser Deployed at Averam" / "The new Carrack Light Cruiser, Carrack Light Cruiser 1, has been deployed at Averam." | **fixed**: the body named the class twice before |
| 12, 13 | after Confirm, the troops band reads "Destination: Victory Destroyer 2": the regiment was placed in a ship of the fleet | **open**: the port carries regiments by fleet, not by ship, and the band names Fleet 1. Which ship the planner picks (`FUN_00552300` → `FUN_00552b10`, `FUN_0054fbb0`) is not traced: it took the second ship, not the first |
| 15, 16, 17 | the regiment is aboard Fleet 1; "Imperial Army Regiment Deployed to Averam" / "The following units have been deployed to Victory Destroyer 2" | the title and the boarding match; **open**: the body names the system |
| 28..33 | a band dragged onto the fleet sets the Destination too | **open**: the port has no band drag (`FUN_00422ce0`, window type 9) |
| 34 | a band dropped on a ship entry names that ship | **open**: no ship Destination and no band drag |

**The gate.** Run `build-selection-2026-11-21T04-10-28-051Z-207892` passed
on both sides. It used Linux and Arch Chromium 157.0.8103.78 through a
temporary `.gate-build-selection-linux.mjs` copy. The test WASM was
cargo-built (sha256 `b1ed55a451f1d8f0…`), and `result.json` has sha256
`9dda1acdd0279488…`.

- The ships and troops bands released on the fleet read "Destination:
  Fleet 1". Their line ink is compared with A0 04 and 07, band-relative:
  ships [6,61,93,66] against [5,60,92,66], troops [6,61,92,66] against
  [5,60,92,66], within 2 pixels and 3 on the right.
- The facilities band is refused with `reason=0x25`.

The cell stays partial while the regiment's ship, the message's ship, the
band drag and the ship Destination are open.

## Destination released on a fleet, corrected (OBJ-07 C019, 2026-11-21)

The 2026-11-03 section below read `FUN_00537180` wrongly. Its range test is
`0xa0 <= family < 0xa2`, so only the construction managers (`0xa0..0xa1`,
"Facilities Under Construction", `FUN_0044acd0`) go through `FUN_00553410`
and must name a system (else `1/0x25`). A shipyard manager (`0xa2..0xa3`)
or training manager (`0xa4..0xa5`) takes a system, a fleet (`0x08..0x0f`)
or a capital ship (`0x14..0x1b`), provided the destination is completed,
not en route, and not destroyed. The planner `FUN_00552300` then tries the
fleet itself first, so the ordered ships wait in that fleet. On completion
they travel from the yard to the fleet's system (`FUN_005150c0`,
`FUN_00556430`; `FUN_00555b30` takes each end's system).

**What the A0 captures show.** In `build-selection2/30` and `32`, the
Empire's ships band was released on Fleet 1 at Yavin. Yavin is the
Alliance's system (red title), and Fleet 1 is the Alliance's fleet. In
`35`, the release point in Selonia's Fleet window shows no fleet. So the
three captures show the other side's fleet refused, and no release on the
player's own fleet. Which check refuses the other side's fleet is not
traced.

**The port now.**
- A ships or troops band's Destination released on a fleet of its own side
  is set (`ManufacturingState::set_fleet_destination`; logged
  `status=set areas=1 to_fleet=NAME`). The band reads `Destination: Fleet
  N`, and the products are placed in the fleet (`products::reconcile`).
- The facilities band, and the facility icon (whose team holds every
  manager), are refused with `0x25`.
- The other side's fleet is refused (logged `reason=other_side`).
- Save v138 keeps the fleet Destinations and each product's fleet target.

**Port deviation.** When the fleet has no room for a squadron or regiment,
the original refuses the Build at Confirm (`0x27`). The port places the
unit at the fleet's system instead.

**The gate.** The `fleetDestination` step (fixture 96) now does the following:

- releases the ships band's Destination on the player's fleet;
- asserts the log and the band line `Destination: Fleet 1`;
- releases the facilities band's Destination on the same fleet;
- asserts `reason=0x25` and an unchanged facilities band.

On the base, the first assertion fails: the base WASM logs a refusal for
the ships band.

Run `build-selection-2026-11-21T01-06-50-261Z-1776891` (Linux, Arch
Chromium 157.0.8103.78 through a temporary `.gate-build-selection-linux.mjs`
copy, not the pinned build) passed on both sides. The test WASM was built
with `cargo build --release --target wasm32-unknown-unknown -p
rebellion-app --features interface-test-fixtures` into the worktree's site,
without wasm-opt (sha256 `98f59136770a49c5…`). `result.json` sha256
`18ab7c5898a99a57…`. Screenshots: `fleet-destination.png`,
`fleet-destination-facilities.png`. An earlier run in the same session
(`…T00-58-51-823Z-1634139`) failed in `facilityRoomCase` (8 against 6
classes walked). That run overlapped a `cargo mutants` run, and the rerun
passed.

The A0 captures show no release on the player's own fleet, so acceptance
of an own fleet rests on the decompile alone. The cell stays partial until
an A0 capture shows it.

## Destination released on a fleet is refused (OBJ-07 C019, 2026-11-03)

**A0.** In `build-selection2/30`, `32` and `35`, a band's Destination was released on Fleet windows: Yavin's Fleet 1, then Selonia's fleet. Each time the band kept its Destination.

**The source.** `FUN_00422ce0` asks the Fleet window's `+0x70` (`FUN_004a3130`) for the target. That is a fleet, a ship, or nothing.

The 0x214 order's checks, `FUN_00537180`, treat a production manager (family 0xa0..0xa2) in two steps:

1. They call `FUN_00553410` on the destination.
2. They require a system (0x90..0x97). Anything else fails with `1/0x25`.

So a fleet is refused.

**The port before.** The port took the fleet's system as the Destination.

**The port now.** A fleet target is refused: the band keeps its Destination, and the log reads `status=refused reason=0x25`.

**The gate.** Fixture 96, `BuildSelectionFleetDestination`, moves a player's fleet to the secondary system and opens that system's Fleet window beside the Manufacturing window. The build-selection gate's `fleetDestination` step:

- chooses Destination from the ships band;
- clicks the fleet;
- asserts the refusal and the unchanged band.

Before the fix, the step failed (no refusal logged, Alliance). After it, it passed on both sides: run `build-selection-2026-11-03T05-33-09-575Z-2234554`.
