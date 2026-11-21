# Production Destination

Recovered 2026-10-06 from REBEXE.EXE with Homebrew Ghidra 12.1.3 (read-only,
no analysis), with STRATEGY.DLL's bitmaps, TEXTSTRA and the manual. Every
function named here has a `FUN_<address>.c` note in this directory. See also
`build-delivery.md` (transit mechanics) and `sector-icon-menus.md` (the
facility-icon menu).

The manual (pp. 44-46, 84, 128; Figs. 2.41, 2.42, 3.58, 3.75-3.76) gives
the player's view:

> DESTINATION: Set the destination for the trooper regiment being trained.
> (p. 128)

Fig. 3.75 (Manufacturing and Production window) shows the context menu with
"Destination" as the first item, and "Reserved" as the last. Fig. 2.42
(Build Selection window) shows "Best Time to Completion: Days: 8" and
"Best Time to Deployment: Days: 88" as separate fields.

## The order kind `0x214` (Destination)

From `sector-icon-menus.md`, the facility icon's right-click menu lists
Destination (`0x214`, TEXTSTRA 12290, sort 120) and Reserved (`0x216`,
TEXTSTRA 12294, sort 1002) among its orders.

The order resolves its team through `FUN_00512700(system, order, kind=4,
out)`. For kind 4 with Destination (`0x214`) or Reserved (`0x216`), the
team is the system's objects of family `0xa0..0xaf` (`FUN_0052c170`).

## The `0xa0..0xaf` family

`FUN_0052c170` is a constructor that initializes an iterator over family
`0xa0..0xaf`:

```c
FUN_0052c170(this, param_1, param_2):
    local_14 = 0xa0;
    local_10 = 0xb0;
    FUN_00513050(this, param_1, {0xa0, 0xb0}, param_2);
    vtable = PTR_FUN_0065fe54;
```

This constructs a range iterator for objects whose family byte is
`0xa0..0xaf`. These objects appear in a system's child list alongside
facilities, fleets, and regiments.

From `build-delivery.md`, the manufacturing manager (the facility-side class
at vtable `0x0065fbf8` and `0x006629a0`, constructors `FUN_005279d0` and
`FUN_0055b2f0`) keeps a queue at `+0x54 -> +0x18`. The manager has a
per-system production-area identity. hyp: the `0xa0..0xaf` objects are
**manufacturing managers** -- per-production-area manager objects that exist
one per facility type per system (e.g., one for the shipyard, one for the
training yard, one for the construction yard). Each manager tracks its
queue, current product, progress, and deployment destination.

Evidence:
- `FUN_00512700` returns them for kind 4 (facility icon), not for kind 8
  (defense) or 0x10 (fleet).
- The Destination order and Reserved order both act on these objects.
- The build-delivery note's manager keeps `+0x74` (DeploymentKey), `+0x5c`
  (progress), `+0x68` (cost).
- Family `0xa0..0xa4` appears in the vtable list alongside facility families
  (`0x20..0x2d`, `0x80`).

## How the player picks a destination

### From the sector window's facility icon menu

1. Right-click the facility icon (kind 4) in the sector window.
2. Select "Destination" (`0x214`).
3. hyp: the cursor changes to targeting crosshairs (the same targeting mode
   as Mission and Build orders, `FUN_00422ce0`'s `+0xc0` mode). The player
   clicks a system on the galaxy map or a system's Defense/Fleet window.
4. The target system becomes the destination for all products currently
   queued at that production area.

### From the Build Selection window

The Build Selection window (Fig. 2.42) shows "Best Time to Completion" and
"Best Time to Deployment" as separate lines. "Best Time to Deployment"
includes transit time from the manufacturing system to the destination.

## Where the destination is stored

From `build-delivery.md`:
- Each manufactured product carries `DestinationLocationAtDeparture` (key at
  `+0x3c`, setter `FUN_004f7120`, notifier `FUN_004fbe50`).
- `DestinationCount` (arrival tick at `+0x44`, setter `FUN_004f7390`,
  notifier `FUN_004fbf10`).
- The manager itself has a slot `+0xc` (`slot_0c()`) that returns its
  location key, and the product has the same slot.

When a product completes (`FUN_0052bee0`):
```c
a = mgr->slot_0c();    // the manager's location key
b = product->slot_0c();  // the product's destination key
set_deployed(product, 1);
if (a != b && a && b) set_enroute_active(product, 1);  // starts transit
set_completed(product, 1);
```

hyp: the Destination order sets the manager's destination (or each queued
product's `+0x3c` key) to the target system's key. New products queued after
the Destination is set inherit the same destination. The product's
`slot_0c()` returns this destination, and at completion the game compares it
to the manager's location to decide whether transit is needed.

## How new queued products inherit the destination

hyp: when the player queues a new build order, the product object is created
with its `+0x3c` (DestinationLocationAtDeparture) set to the current
destination of the production area. This is either the system where the
facility is located (default, no transit needed) or the target system the
player selected with the Destination order.

The manager's `+0x74` (DeploymentKey) may serve as the persistent per-area
destination that new products copy.

## "Best Time to Completion" and "Best Time to Deployment"

From the Build Selection window (Fig. 2.42):
- **Best Time to Completion**: the time until the product is finished
  building. This is `cost(+0x68) - progress(+0x5c)` ticks, converted to
  days by the facility's processing rate (the `0x394` timer period at
  facility `+0x54 + 0x20`, per `build-delivery.md`).
- **Best Time to Deployment**: completion time plus transit time from the
  facility's system to the destination system. Transit time uses
  `FUN_0055d8c0(coords(from), coords(to), speed)`:
  ```c
  d = isqrt(dx*dx + dy*dy);
  t = ((d / GNPRTB[5120]) * speed) / 100;
  return t == 0 ? 1 : t;
  ```
  with GNPRTB 5120 = 5 and speed = the product's hyperdrive value.

hyp: the displayed values are in absolute days from the current tick, not
relative. "Days: 8" means the product finishes in 8 game days; "Days: 88"
means it arrives at its destination in 88 game days.

## Reserved (`0x216`)

TEXTSTRA 12294 "Reserved". The STRATEGY record has bitmap 11902 (the check
mark) at word 10, indicating it is a toggleable state. From the manual
(p. 86):

> Reserve the construction yard. This means that if you turn over
> Maintenance Production to your agent, the agent won't use that facility
> to build mines and refineries. You might do this if you want to keep a
> construction yard available for your own purposes.

hyp: the Reserved flag is stored on the `0xa0..0xaf` manager object, and the
AI's Manage Production/Manage Garrisons agent skips reserved managers.

## Port notes

- The Destination order acts on `0xa0..0xaf` manager objects. port: the
  port keeps one destination per system and production area
  (`ManufacturingState::set_destination`) in their place.
- The facility icon's menu enables Destination when the system has a
  manufacturing facility of the player's (port: its production areas). The
  targeting release on a planet sets every area's destination
  (`PanelAction::SetDestination`), logged as `[interface] command=0x214
  destination=production_destination status=set`. A release on a fleet
  (2026-11-21): `FUN_00537180` holds only the construction managers
  (`0xa0..0xa1`) to a system (`move-order.md`, "Destination checks"), so
  the icon's team, which holds every manager, is refused with `1`/`0x25`,
  while a ships or troops band's own manager takes a fleet of its side
  (`ManufacturingState::set_fleet_destination`, save v138; logged
  `status=set areas=1 to_fleet=NAME`). The products are then placed in the
  fleet (`FUN_00552300` tries the target first; `products::reconcile`),
  travel to its system when finished, and the band names the fleet. A
  Build whose squadrons or regiments find no room in the fleet, or in the
  ship it names, is not valid (2026-11-21, "Build with no room aboard"
  below; `build_selection::lacks_fleet_room`). The other side's fleet is
  refused (A0 `build-selection2/30`, `32`); the refusing check is not
  found ("The other side's fleet" below). The Build agent's fleet target (`agent_build.rs`) still names
  the fleet's system. Its Reserved is ported since 2026-11-10
  (`sector-icon-menus.md`, "The facility icon's Reserved"), and so is its
  drop, a `0x214` against the drop window's `+0x68` ("Drags by kind"
  there).
- A0 `2026-11-21-build-dest-16` (Empire, own Fleet 1 at Averam):
  - ships (`04`, `33`) and troops (`07`, `31`) bands take the fleet, by the
    menu's targeting or a band drag; the facilities band is refused
    silently (`10`, `29`);
  - a regiment ordered for Fleet 1 names a ship of it, "Destination:
    Victory Destroyer 2" (`12`), and its message body "deployed to Victory
    Destroyer 2" (`17`, title "... Deployed to Averam"). The planner
    (`FUN_00552300`) weighs the fleet, then each of its ships
    (`FUN_00552b10`), and keeps a later candidate only when
    `FUN_0054fbb0` finds it strictly better: no refusal first, then more
    room at record `+8`. `CapShip_vt78` (`FUN_00500b40`) gives that room:
    the troop capacity (`+0x270`) less the regiments aboard
    (`FUN_00504c40`, mode 3), or the fighter capacity (`+0x26c`) less
    the squadrons (`FUN_005039d0`). With an Imperial Star Destroyer
    (troop capacity 3) and a Victory Destroyer (2), four regiments seat
    ISD, ISD, VD, ISD, so the fifth goes to the Victory Destroyer.
    Ported 2026-11-21: `products::seat` (save v138 keeps each product's
    seat). port: the port keeps a fleet's regiments as one list, so
    `products::seat_regiments` seats the carried ones again in order by
    the same rule;
  - a Carrack ordered for Fleet 1 stands in Fleet 1 at once with the
    under-construction art (`23`), finishes there (`25`): "The new Carrack
    Light Cruiser, Carrack Light Cruiser 1, has been deployed at Averam."
    (`27`); the body names the ship's own name (ported 2026-11-21,
    `story_messages::deployed_texts`);
  - each band names its own system again once its last unit completed
    (`14`, `25`), while a band that never built keeps the fleet (`18`):
    `ManuMgr_vt204` (`FUN_0052b8c0`), the RemainingGameObjCount hook,
    sets the TargetKey to none when the count reaches 0 (ported in
    `ManufacturingState::add_point`; a Stop reaches it too, "Stop resets
    the Destination" below);
  - a band dragged (`28`..`33`): `FUN_004534f0` (the window's procedure)
    starts a drag on a move with the button held inside the bands'
    rectangle (8, 0x39)..(0xe6, 0x11a). The drag image is the window's
    `+0x254 + 4 * (n - 1)` for `n` selected bands: STRATEGY 0x2841..0x2843
    for the Alliance and 0x2844..0x2846 for the Empire (`FUN_00456230`),
    with hotspot (3, 3). A release 4 or more pixels from the start posts
    `0x29a` to the main window, and `FUN_00422ce0` gives window type 9 a
    `0x214` against the `+0x70` object. Ported 2026-11-21
    (`SystemWindowAction::DropBand`);
  - a band dropped on a ship entry of the fleet panel names that ship
    (`34`): `FUN_00537180` takes a capital ship (`0x14..0x1b`). Ported
    (`ManufacturingState::set_ship_destination`, `FleetWindowState::release_ship`).
- Gate: `tools/interface-parity/fleet-window.mjs`, case `destination`
  (fixture code 54, a player shipyard at the primary system), both sides.
- Best Time to Deployment = Best Time to Completion + transit time, where
  transit time is the same formula as fleet/product travel
  (`build-delivery.md`).
- port: the Destination targeting mode (cursor crosshairs, click a system)
  parallels the Mission targeting mode.

## Build with no room aboard (read 2026-11-21)

Build Selection's check (`FUN_005385f0`, the build orders' `vt+0x50`)
asks, per unit, `FUN_00551a20` -> `FUN_00552300` (the planner) for the
manager's TargetKey:

- The planner weighs the target (`FUN_00552b10`). For a system
  (`0x90..0x97`) it then weighs the system's fleets; for a fleet
  (`0x08..0x0f`) each of its ships (`FUN_00502e30`, `FUN_00552b10`); a
  capital ship (`0x14..0x1b`) is weighed alone (`FUN_00552300` lines
  60..120).
- A fleet's room (`Fleet_vt78`, `FUN_004fe160`) is unlimited for capital
  ships (rows `0x14..0x1c`) and nothing for any other row, with its own
  refusal; a ship's (`CapShip_vt78`, `FUN_00500b40`) is its fighter
  (`+0x26c`) or troop (`+0x270`) capacity less what rides it.
  `FUN_00550700` records `{1, 0x27}` for a node that accepts the row but
  has no room and no "unlimited" (`0x5508a2`).
- `FUN_0054fbb0` keeps the first candidate with no refusal and more room;
  with none, the planner copies the kept candidate's refusal to the order
  (`LAB_0055275a`). Each unit then lowers its node's room by one
  (`FUN_00552dd0` -> `FUN_00550eb0`, `FUN_0054ff00`), so the order fits
  while its number is at most the room left in the target.
- The manager's own queued products give their room back first
  (`FUN_00552210` -> `FUN_00552000` -> `FUN_00552dd0` in mode 0), as
  Confirm scraps them.

An order with a refusal is not valid: Build Selection shows "n/a" in the
four values and greys Confirm (`FUN_00438220`, `FUN_00439160`;
`manufacturing-build-selection.md`). Ported 2026-11-21:
`build_selection::lacks_fleet_room` in `build_selection::estimate`, with
`products::ship_rooms_but` for the room left. A0 shows no Build refused
this way; the reading rests on the decompile.

port: `products::seat` still seats a unit a named ship cannot take on
another ship of the fleet, where the original's planner keeps the ship
alone. That only happens when the ship's room drops after Confirm.

## Stop resets the Destination (read 2026-11-21)

Stop (`0x213`) reaches `ManuMgr_vt1f8` (`FUN_00529ba0`), which scraps the
queue with `FUN_0052a2c0`: each product's slot `+0xac` with cause `0x16`
(`GameObj_SetDestroyedReason`, `FUN_004f71d0`, then the destroyed bit
`FUN_004f7560`). The destroyed bit clears the derived existing bit
(`0x004f7bc0`), so event `0x300` (`FUN_005140c0`) runs for each product:
not completed and with a builder (`+0x38`), it calls `FUN_00529cf0` ->
`FUN_0052a710`, which drops the destroyed product and sets
RemainingGameObjCount (`FUN_0052a7d0`). At 0, `ManuMgr_vt204`
(`FUN_0052b8c0`) sets the TargetKey to none, so the band names its own
system again. A manager left without yards scraps through the same
`FUN_0052a2c0` (`FUN_00529dd0`). Build (`FUN_00529ab0`) scraps too, but it
creates the new units (`FUN_0052a140`) before the `0x300` events run, so
the count never reaches 0 and the Destination stays (A0
`2026-11-21-build-dest-16/12`). Ported 2026-11-21 in
`ManufacturingState::stop` (`build` and `build_units` scrap through
`scrap_queue`, which keeps it). A0 shows no Stop on a band with a
Destination elsewhere; the reading rests on the decompile.

## The other side's fleet (read 2026-11-21, not found)

A0 `build-selection2/30`, `32`: the Empire's empty ships band released on
Fleet 1 in Yavin's Fleet window (an Alliance system) keeps "Destination:
Chandrila"; `33`: released on Yavin's planet, it takes Yavin. The checks
read so far do not refuse that fleet:

- The targeting release (`FUN_00422ce0`, `0x214`) takes the window's
  `+0x70` (`FUN_004a3130`: the left list's item under the point, else the
  window's subject) with no side test, sets it (order `+0x2c`) and submits
  (`FUN_0041ce20`).
- `FUN_00537180` refuses `1`/`0x28` for a destination of another side only
  when some product queued at a manager is not a capital ship (the
  manager's `+0x54 -> +0x18` list through `FUN_00528820`, read at
  `0x537326..0x5373e8`). The band was empty.
- Its per-command checks read the manager's `+0x58`, the
  RemainingGameObjCount. At 0 (an empty band) they test only the
  destination's state (`0x20`, `0x21`, `0x22`) and family (`0x25`); a
  fleet passes. With products queued they run the planner for each
  (`FUN_00552210`, `FUN_00551900`, `FUN_00583f50`).
- The route (`FUN_005513a0`) finds the destination in the side's table
  (`FUN_00552e80` -> `FUN_00553130` -> `FUN_004f2d10`, else `1`/`0x22`)
  and its container (`FUN_00552ff0`, else `1`/`0x23`). `Fleet_vt74`
  (`FUN_004fe0d0`) -> `FUN_00553410` gives `{1, 0x24}` for a fleet of
  another side, but `FUN_00550700` writes it to the node's own status
  (`ecx = esp+0x28` at `0x5508ac`), not the order's.
- The command's check (`FUN_0057e0d0`): `ManuMgr_vt1e4` (`FUN_00528650`)
  needs the destination in the ordering side's table (`FUN_004f2d10`,
  else `{1, 1}`), then routes only when `+0x58` is not 0.

Left: whether the other side's fleet is in the side's table at all
(`FUN_0053efd0` -> `FUN_00585dc0`, `FUN_005844e0`), whether `FUN_004a3130`'s
list hit test (`FUN_00609d80`, `FUN_00609d10`) returns that fleet's item,
and whether that Fleet 1 was the Alliance's. port: the refusal stays
(`reason=other_side`).

## Open

- The exact Destination order handler (which function processes `0x214`
  after the player selects a target system). It likely updates the manager's
  destination key and all its queued products' `+0x3c`.
- Whether the Destination order fires a targeting mode or opens a dialog.
- The `0xa0..0xaf` object's full field layout and vtable.
- Whether the Build Selection window computes "Best Time to Deployment"
  at display time from the current destination, or stores it.
- Which `FUN_0051f8f0` factory entry handles kind `0x214`.

## Supporting decompiles

`FUN_0052c170`, `FUN_00512700`, `FUN_0052b960`, `FUN_0052bee0`,
`FUN_0055d8c0`, `FUN_00429440`, `FUN_00604500`.
