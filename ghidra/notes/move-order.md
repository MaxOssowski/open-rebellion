# Move and Confirmed Move

This note records how the player moves a fleet: the Fleet pop-up menu, the
Move (`0x201`) and Confirmed Move (`0x202`) orders, the targeting release, the
per-object move command, and the confirmation window. The menu window and the
targeting mode are shared with Mission (`object-popup-menu.md`).

The manual (pp. 121-122): "Issue commands to fleets by right-clicking on the
Fleet icon. This brings up the Fleet menu." Move: "the cursor changes to
targeting cross hairs. Click on the destination system or press the ESC key to
cancel. The fleet immediately goes into hyperspace to reach its destination."
"Any time a fleet, or a ship within a fleet, is in hyperspace, it cannot
receive orders." Confirmed Move "brings up a window that tells you the transit
time (in days) it will take for the fleet to reach its destination. To confirm
the move, click the checkmark. To cancel, click the X button."

## Object families

Slot `+4` of an object returns its family, the top byte of its id. The DAT
headers give the ranges (words 2 and 3 of each `*SD.DAT`):

| Families | File | Objects |
|---|---|---|
| `0x08..0x0f` | FLEETSD | Fleets |
| `0x10..0x13` | TROOPSD | Regiments |
| `0x14..0x1b` | CAPSHPSD | Capital ships |
| `0x1c..0x1f` | FIGHTSD | Fighter squadrons |
| `0x20..0x21` | ALLFACSD | hyp: facility placeholders (two records) |
| `0x22..0x27` | DEFFACSD | Defense facilities |
| `0x28..0x2b` | MANFACSD | Manufacturing facilities |
| `0x2c..0x2f` | PROFACSD | Production facilities |
| `0x30..0x37` | MJCHARSD | Major characters |
| `0x38..0x3b` | MNCHARSD | Minor characters |
| `0x3c..0x3f` | SPECFCSD | Special forces |
| `0x40..0x7f` | MISSNSD | Missions |
| `0x80..0x8f` | SECTORSD | Sectors |
| `0x90..0x97` | SYSTEMSD | Systems |

`FUN_004025b0(object)` returns the object's own id, built as
`family << 24 | +0x18`; `object[7]` is its container.

## The Fleet menu

The fleet class is vtable `0x0065d438` (constructor `FUN_004fd650`, factory
`FUN_004fe710`). Its `+0x3c` slot, `0x004fdad0`, copies a static list that
`FUN_004ff8e0` builds once in `DAT_006b2b10`: `0x201`, `0x202`, `0x204`,
`0x203`, `0x220`, `0x221`, `0x222`, `0x223`, `0x234`, `0x200`. Slot `+0x58`,
`FUN_004fdb00`, accepts a kind only when the list holds it (status `1`/`0x17`
otherwise). `FUN_0051d990` then adds Encyclopedia (`0x100`) and Status
(`0x103`) as for every selection (`object-popup-menu.md`).

The STRATEGY.DLL 26-byte `RT_RCDATA` records (`FUN_00442790`; words 0 kind,
1 parent, 2 sort, 4 submenu flag, 5 TEXTSTRA id, 10 second bitmap, 12 module):

| Kind | Parent | Sort | Submenu | TEXTSTRA | Text |
|---|---|---|---|---|---|
| `0x201` | — | 10 | 0 | 12312 | Move |
| `0x202` | — | 12 | 0 | 12311 | Confirmed Move |
| `0x120` | — | 200 | 1 | 12313 | Planetary Bombardment |
| `0x220` | `0x120` | 202 | 0 | 12314 | Target Military Facilities |
| `0x221` | `0x120` | 204 | 0 | 12315 | Target Civilian Facilities |
| `0x222` | `0x120` | 206 | 0 | 12316 | General Bombardment |
| `0x223` | `0x120` | 208 | 0 | 12317 | Destroy System |
| `0x234` | — | 220 | 0 | 12318 | Planetary Assault |
| `0x203` | — | 500 | 0 | 12291 | Rename |
| `0x100` | — | 1000 | 0 | 12292 | Encyclopedia |
| `0x103` | — | 1001 | 0 | 12293 | Status |
| `0x200` | — | 2000 | 0 | 12295 | Scrap |

`0x204` has no record, so it never becomes an item; it is the per-object move
(below). No record carries an icon (words 7 and 8 are zero). The sorted order
matches the manual's Fig. 3.64: Move, Confirmed Move, Planetary Bombardment,
Planetary Assault, Rename, Encyclopedia, Status, Scrap. The menu is built and
opened by the owning window as in `object-popup-menu.md` (`FUN_00442590`,
`FUN_00442380`); the system window's fleet list is that owner.

## The orders

`FUN_0051f4b0` registers the factories (`FUN_0051f930`): `0x201` is
`FUN_0053cec0` → `FUN_0053ce70` (vtable `0x00661850`), `0x202` is
`FUN_0053cf90` → `FUN_0053cf40` (vtable `0x006618a8`). Both are 0x4c bytes over
the move base `FUN_0053c040` (vtable `0x006617f8`, over the order base
`FUN_0051fa20`), which allocates a sub-order list at `+0x44` and a status pair
at `+0x48` (`FUN_0053c710`). The two vtables differ only in slot `+0xc`, the
kind (`FUN_0053cf30` returns `0x201`, `FUN_0053d000` returns `0x202`).

| Slot | Function | Role |
|---|---|---|
| `+0x10` | `FUN_0040f340` | Listed: always 1 |
| `+0x14` | `FUN_0051fd30` | Checked: always 0 |
| `+0x18` | `FUN_0053c100` | Enabled: `FUN_0053c4b0`, then every sub-order's `+0x18` |
| `+0x1c` | `FUN_0053c1a0` | Validator: `FUN_0053c4b0`, then every sub-order's `+0x1c` |
| `+0x20` | `FUN_0053c240` | Execute: every sub-order's `+0x20` |
| `+0x24` | `FUN_0051fb70` | Set the team (list at `+0x2c`) |
| `+0x28` | `FUN_0051fb80` | The team list (`this + 0x2c`) |
| `+0x2c` | `FUN_0051fbb0` | Set the target (`+0x34`) |

Statuses are two words, `(1, -1)` meaning success (`FUN_00520580` copies;
`object-popup-menu.md`, `mission-dialog.md`).

### Expansion into sub-orders

`FUN_0053c4b0` fails with `1`/`1` unless `+0x44` and `+0x48` exist, then:

1. `FUN_0053f150` resolves each team id through its class handler
   (`FUN_0053efa0`, slot `+0x2c`) into the objects that will move, without
   duplicates. An id with no handler gives `1`/`0x12`; no object at all gives
   `1`/`0x16`.
2. `FUN_0053c810` repeatedly takes the characters and special forces
   (`FUN_0053cbc0`, families `0x30..0x3f` resolved through `FUN_00504dc0`,
   `1`/`0x12` when one does not resolve) and builds one `0x241` order for
   them (`FUN_004f5440`, vtable `0x0065d170`, over the mission order base
   `FUN_004f4690`): side, team, and target copied from the move.
3. `FUN_0053ca50` builds one `0x204` order for the remaining objects
   (`FUN_0053d020`, vtable `0x00661900`, over `FUN_0051fa20`): side, team, and
   target copied from the move.

Each sub-order goes into `+0x44` (`FUN_00536fe0`).

### The per-object move `0x204`

| Slot | Function | Role |
|---|---|---|
| `+0xc` | `FUN_0053d0e0` | Kind `0x204` |
| `+0x18` | `FUN_0051fe20` | Enabled: built, then `+0x4c` |
| `+0x1c` | `FUN_0053d100` | Validator: `+0x4c` (`FUN_0051ff30`), then `FUN_00553aa0` |
| `+0x48` | `FUN_0053d3c0` | Create command `0x201` (`FUN_00578ab0`, vtable `0x00669558`) |
| `+0x4c` | `FUN_005201c0` | The command's checks through `FUN_005535b0` |
| `+0x50` | `FUN_0053d430` | Destination checks for the whole group |

`FUN_00553aa0` turns a status `1`/`0x25` or `1`/`0x27` into `1`/`0x26` when
the target is one of the team (`FUN_004f5940`).

`FUN_0053d430` resolves the destination (`FUN_00504e60`, `1`/`0x22` when it
does not resolve) and the side (`FUN_00553b80`). It then refuses `1`/`0x28`
when all of the following hold:

- the destination's side bits (`+0x24` bits 6..7) differ from the order's side;
- some member is neither a fleet (`0x08..0x0f`) nor a capital ship
  (`0x14..0x1b`);
- and either some member is not a regiment (`0x10..0x13`), or the
  destination is not a system with `+0x50` bit `0x40` set and `+0x88` bit 1
  clear.

So fleets and capital ships may move to an enemy or neutral destination; a
group holding anything else may not, except an all-regiment group landing on
such a system (hyp: an invasion). A fleet-only move never meets this
refusal. Then it plans the route (`FUN_00551190`, `FUN_005513a0`, below) and
gives each member its leg (`FUN_00551630`, stored at the member's `+0x48`).
Any failure gives `1`/`1`.

### The move command `0x201` (vtable `0x00669558`)

The command holds the object at `+0x3c`, a route context at `+0x40`, and the
destination at `+0x48`.

- Can move, `+0x18` (`FUN_00578c00`):
  1. The object must resolve (`1`/`0x12`).
  2. The object's slot `+0x6c` must pass. For a fleet that is
     `FUN_004fdc70`, which runs the object check `FUN_004f9860`
     (`object-popup-menu.md`, "When Mission is enabled": side, untraced, en
     route, and the rest) and gives `1`/`1` on failure.
  3. `FUN_005152e0` → `FUN_005555e0` must pass. It refuses `0x90`/4 when an
     ALLFACSD object (`0x20..0x21`) sits in a system with `+0x88` bit `0x20`
     set, its blockade bit (`blockade-troop-withdrawal.md`), and
     `FUN_0055a020(ships, fighters)` over that system's active fleets comes
     to less than 31. It does not apply to a fleet.
- Validate with a destination, `+0x1c` (`FUN_00578d00`):
  1. The galaxy must exist (`FUN_00506e60`).
  2. The object must resolve (`1`/`0x12`), and so must the destination
     (`1`/`0x22`).
  3. The object's `+0x70` must pass (for a fleet, `FUN_004f6800`, which runs
     `+0x6c` again).
  4. The route must hold (`FUN_005542f0` → `FUN_00551190`, `FUN_00554490`).
  5. `FUN_00515390` → `FUN_00555920` must pass:
     - `FUN_00555410` → `FUN_00555460` reports whether the object and the
       destination lie in the same system (`FUN_00555540` on both);
     - in different systems, the object's speed slot `+0x34(1)` must be
       non-zero, or the result is `1`/`0x18`. For a fleet that is
       `FUN_004fd900`, zero when a member capital ship has no hyperdrive
       (`build-delivery.md`): the fleet cannot enter hyperspace;
     - a regiment (`0x10..0x13`) whose destination's side bits differ gets
       `1`/`0x28`; this does not apply to a fleet;
     - any other failure gives `1`/`1`.
- Execute, `+0x20` (`FUN_00578f30`): validate again, then `FUN_00515440` →
  `FUN_00556390(object, destination, ctx)`. This finds the leg
  (`FUN_00555410`) and, unless the object is autorouting (`+0x50` bit
  `0x800`), departs through `FUN_004f7640`, which sets the in-transit bit
  (`+0x50` bit 5; `blockade-troop-withdrawal.md`, "Departure roll").
  Arrival follows the per-object transit in `build-delivery.md`.

So a fleet in hyperspace cannot take a new Move: `FUN_004f9860` refuses an
object with `+0x50` bit 4 (en route).

## Choosing Move or Confirmed Move

`FUN_004ac730` (the system window's `+0x20`) calls `FUN_0041cdf0(kind,
selection, no target, 0)`, which reaches `FUN_00486fb0`. For `0x201` and
`0x202`, `FUN_00487c50` builds the order (side `+0x20`, team `+0x24`,
`+0x2c`). With no target (top byte of the target id zero) the order goes to
`FUN_0041d5e0` → `FUN_00429320`: the galaxy view's targeting mode, cursor
1002, exactly as Mission (`object-popup-menu.md`, "Targeting").

## Hit tests

The galaxy view keeps its modeless child windows in the list at `+0x6c`.
A window's id is `(index << 6) | type` (`FUN_0045aac0`), and the drag path
reads the type as `+0x24 & 0x3f`. Each class answers two hit tests with a
point in its client coordinates: `+0x68` (the object under the point) and
`+0x70` (the container under the point, used for move orders).

| Type | Class | Vtable | `+0x70` | `+0x68` |
|---|---|---|---|---|
| 1 | Sector window, `FUN_004591d0` (opened by `FUN_00429ce0` from a click on a sector) | `0x00659f18` | `FUN_0045c830`: the item in list `+0x164` whose rectangle (`+0x40`) holds the point (`FUN_0045c660`), else no id | `FUN_0045c6b0`: the same item, or the overlay item in list `+0x174` (flag `0x10000`) whose bitmap pixel is hit (`FUN_005fca00`) |
| 9 | System window, `FUN_00452fc0` | `0x00659e68` | `FUN_004aa470`: the window's system (`+0x144`) wherever the point is | `FUN_00458b80`: the list item under the point (list `+0x174`, offset (8, 77)), else the system; on page `0x67` always the system |
| 4 | hyp: fleet window, `FUN_004a2630`, 235 by 304 | `0x0065bda0` | `FUN_004a3130`: the item under the point in the left list (4..95, 29..295, list `+0x160`) or, when `+0x194` is `0x66`, the right list (101..234, 127..291, list `+0x164`); elsewhere the window's subject `+0x144`, or the single selected item | `FUN_004a2f80` (not read) |
| 10 | `FUN_004a7790` | `0x0065be30` | `FUN_004aa470`: the window's subject `+0x144` | `FUN_004aa380`: the item under the point (list `+0x164`, offset (7, 81)), else the subject |
| 11 | `FUN_0049f130`, 235 by 304 | `0x0065bd10` | `FUN_004aa470`: the subject | `FUN_004a09a0`: the picture at (108, 37) gives `+0x1c8`, the list `+0x1c0` gives the item under the point, else the subject |

`FUN_0045aac0` opens types 9, 10, 4 and 11 for sector-window items of kind
`4`, `8`, `0x10` and `0x40` (item `+0xc >> 16`); hyp: kind 4 is a system and
`0x10` a fleet. Type 7 (`FUN_0049ee20`, vtable `0x0065bcb8`) has no hit-test
slots there.

The galaxy map is drawn by the galaxy view itself, not by a child: mode 1
finds the sector under a click with `FUN_004420b0`. A targeting release over
the bare map finds no child window (`FUN_00604540` fails for the view and its
parent), so without Shift the order is destroyed.

So, for a move:

- the bare galaxy map gives no destination; the order is dropped;
- a sector window gives the system under the point, and nothing between
  systems;
- a system window gives its own system wherever the release lands, even over
  a fleet in its list, so it never joins a fleet;
- a fleet window (hyp, type 4) gives the item under the point or the window's
  fleet, so a release there joins that fleet;
- the route then accepts only a system, a fleet or a capital ship as the
  destination (`FUN_005531b0`, below).

## The release

`FUN_00422ce0`'s `WM_LBUTTONUP` (0x202) in mode 2:

1. Finds the child window under the point (`ChildWindowFromPointEx`,
   `FUN_00604540`). With Shift held (`param_3 & 4`) it forwards the click to
   that window and stays in mode 2, as for Mission.
2. For orders `0x201`, `0x202` and `0x214` it asks the child's vtable `+0x70`
   for the container under the point; other orders ask `+0x68` (Hit tests,
   above).
3. When the id is non-zero and valid (`FUN_004ece60`), it sets the target
   (`+0x2c`) and calls `FUN_0041ce20(order, 0)` → `FUN_00436050` →
   `FUN_00487740(order, 0)`. Otherwise the order is destroyed.
4. Mode returns to 1 and the capture is released.

`FUN_00487740(order, force)`:

1. Runs the validator `+0x1c`. A failure goes to `FUN_00487c90`, the side's
   advisor reaction (`mission-dialog.md`, "Refusal"), and destroys the order.
2. With `force` 0, `FUN_00487cc0` decides whether to confirm. It confirms when
   the validator passes and the kind is `0x202`, or `0x200`, `0x213`, `0x242`
   or `0x250`. For `0x201` it confirms only when the first team member's
   container is a system whose `+0x88` bit `0x20` (blockade) is set and whose
   side bits equal the member's. Confirming hands the order to `FUN_0048a340`
   (`+0x6c`, below).
3. Otherwise `FUN_00488030` plays the acknowledgement for capital-ship targets
   (families `0x30..0x37`, sound by class through `FUN_004c4990`), and
   `FUN_0048aa90` → `FUN_0041cee0` → `FUN_004360f0` submits the order for its
   side.

So Move departs at once unless the fleet runs a blockade, and Confirmed Move
always asks.

## The confirmation window

`FUN_0048a340` builds a 0x60-byte confirmation (`FUN_0049a1b0`, vtable
`0x0065bbd0`, over `FUN_004c4d90`) holding the order at `+0x5c`, and
`FUN_0041d7b0` → `FUN_0042a590` posts `0x468` with type `0x10` to the galaxy
view. The view's `0x468` handler builds the window `FUN_0044f060` (0x124
bytes, vtable `0x00659cd0`, `FUN_00606380` with size 0x1a8 by 0x14b, 424 by
331).

`FUN_0049a350` fills the confirmation for `0x201`/`0x202`:

- Its text `+0x50` is TEXTSTRA `RT_RCDATA` 0x7057, "Transit time in days".
  When the first member's system is blockaded (`+0x88` bit `0x20`) it is
  0x7056 instead: "Units evacuating from worlds under blockade risk being
  destroyed by blockading vessels.  Are you sure you want to proceed with
  the evacuation?\nTransit time in days". `FUN_0060b840` loads these templates
  from module 2.
- Its picture id `+0x2e` is 1018 for side 1 and 1019 otherwise, or 1031 for
  side 1 and 1030 otherwise when blockaded.
- `FUN_0049a8b0` appends one line per moving object: its name
  (`FUN_004f6270`), `":  "` (`DAT_006a8798`), and its transit days, which
  `FUN_0053c2e0` collects from the sub-orders (`FUN_0053d200` for `0x204`,
  `FUN_004f4850` for `0x241`). hyp: `FUN_00615f00(stream, 10)` starts each
  line with a newline.

`FUN_0044f180` (slot `+0x38`) lays the window out:

| Element | Rect or position | Source |
|---|---|---|
| Background | full window | STRATEGY 11125 when the galaxy view's side `+0x9c` is 1, otherwise 11126 |
| Placement | in the galaxy view's rectangle | `FUN_00606980` with the view's `+0xcc..+0xd8` |
| Text box | (24, 242) 322 by 70 | `FUN_0041ecf0`, the confirmation's `+0x50` |
| Title | (21, 14) | `FUN_00601880`, the confirmation's `+0x44` (empty for moves) |
| Checkmark, control `0x14` | (355, 244) 51 by 35 | `CoolStrobeButton`, STRATEGY 10926 / 10927 |
| X, control `0x15` | (355, 281) 51 by 35 | STRATEGY 10929 / 10930 |
| Picture | (12, 30) | STRATEGY `+0x2e`: 1018, 1019, 1030 or 1031 |

All these bitmaps are staged in `data/base/ui/strategy-dll/BMP/`.

The window's command slot `+0x48`, `FUN_0044f5e0`, handles the two controls:

- Control `0x14` (the checkmark) calls `FUN_0041ce20(order, 1)`. The order
  passes through `FUN_00487740` again with `force` 1: it is validated (a
  refusal still reaches the advisor) and submitted without asking.
- Control `0x15` (the X) destroys the order.

Either way the window closes (`+0x30`). Its key slot `+0x4c`, `FUN_0044f640`,
maps Enter (0xd) to `0x14` and Escape (0x1b) to `0x15`.

## A drag is a move

`FUN_006083c0` posts notification `0x29a` when a list drag ends outside its
list; the galaxy view (`FUN_00422ce0`, default `WM_COMMAND` path) asks the
source window for its selection (`+0x58`), hit-tests the drop point with
`+0x68` and `+0x70`, and issues `0x201`, or `0x202` with Ctrl held
(`GetAsyncKeyState(0x11)`), through `FUN_0041cdf0(kind, selection, target, 0)`:

- Window type 1 (`+0x24 & 0x3f`): members whose list entry has flag 4 are
  split off into a `0x214` order (Destination) against the `+0x68` object;
  the rest move against the `+0x70` object.
- Types 4 and 10: the whole selection moves against the `+0x70` object.
- Type 9: a `0x214` order against the `+0x70` object.

With a target, `FUN_00486fb0` skips targeting and goes straight to
`FUN_00487cc0` (confirm) or `FUN_00487740(order, 0)`.

## Route refusals

The route context (`FUN_00551060`) is set up by `FUN_00551190` →
`FUN_00551100` for the order's side and checked by `FUN_005513a0` (and
`FUN_00551270` per object, `FUN_00554490`). A fleet move refuses to route
when:

- the destination does not resolve, `1`/`0x22` (`FUN_00552e80` →
  `FUN_00553130`);
- the destination's container does not resolve, `1`/`0x23` (`FUN_00552ff0`,
  `FUN_0050c640`);
- the destination is not a system (`0x90..0x97`), a `0x98..0x9f` object, a
  fleet (`0x08..0x0f`) or a capital ship (`0x14..0x1b`), `1`/`0x25`
  (`FUN_005531b0`);
- `FUN_00553aa0` turns `0x25` or `0x27` into `0x26` when the destination is
  one of the team: a fleet cannot move into itself.

Each leg is then built by `FUN_00552d10` (`FUN_00550620`, `FUN_00550700`) and
assigned by `FUN_00551630` (`FUN_005529a0`, `FUN_00552000`, `FUN_00552300`,
`FUN_00552dd0`); those refusals are not read here.

## Escape while targeting

The galaxy view's `WM_KEYDOWN` handles Tab and F1..F7 (`0x70..0x76`) but has
no `VK_ESCAPE` case. The cancel command `0x15e` is posted only by
`FUN_0042db70` (through `FUN_0041db60`), when the global gate
(`FUN_0051de80`, `FUN_0041e390`, `FUN_0041e500`) stops orders. The manual says
Escape cancels Move. hyp: a frame-level key handler maps Escape to `0x15e`;
not found. The confirmation window's own Escape is recovered
(`FUN_0044f640`).

## Port notes

- The port already departs a fleet through `movement::validate_fleet_dispatch`
  and `begin_fleet_transit` (F-007C) with the per-object transit time
  `fleet_transit_ticks` (F-030, `build-delivery.md`). Compare their refusals
  with `FUN_004f9860`, `FUN_005555e0`, `FUN_00555920` and `FUN_0053d430`
  before reusing them.
- The refusal statuses above reach the advisor as in `mission-dialog.md`,
  "Refusal" (for example `1`/`0x18` and `1`/`0x22`).
- Phase 1: a right release on a fleet in the system window's Fleets tab
  opens the Fleet menu (`object_menu.rs`, `MenuObject::Fleet`) with the
  STRATEGY rows above. port: every fleet order is drawn disabled until the
  move order and its targeting are ported; Encyclopedia opens as for any
  single selection.

## Still open

- The names of window types 4, 10 and 11, and `FUN_004a2f80` (type 4's
  `+0x68`); type 4 is read as the fleet window from its size and its opening
  for item kind `0x10`.
- Families `0x20..0x21` (ALLFACSD) and `0x98..0x9f`.
- The leg builders `FUN_005529a0`, `FUN_00552000`, `FUN_00552300` and
  `FUN_00552dd0`.
- What order `0x214` (Destination) does when a system window's selection is
  dragged out (drag type 9).
- `FUN_0053efa0`'s class handlers (`+0x2c`) that turn a team id into moving
  objects.
- Where Escape cancels targeting, if anywhere outside the frame.
- The confirmation window's title text and the meaning of `+0x30`, both unused
  for moves.

## Supporting decompiles

`FUN_00486fb0`, `FUN_00487c50`, `FUN_00487cc0`, `FUN_00487740`,
`FUN_00488030`, `FUN_0048aa90`, `FUN_0048a340`, `FUN_0041cdf0`,
`FUN_0041ce20`, `FUN_0041cee0`, `FUN_004360f0`, `FUN_0041d5e0`,
`FUN_0041d7b0`, `FUN_0042a590`, `FUN_00422ce0`, `FUN_0051f4b0`,
`FUN_0051f8f0`, `FUN_004f5cd0`, `FUN_0053cec0`, `FUN_0053cf90`,
`FUN_0053ce70`, `FUN_0053cf40`, `FUN_0053cf30`, `FUN_0053d000`,
`FUN_0053c040`, `FUN_0053c710`, `FUN_0053c100`, `FUN_0053c1a0`,
`FUN_0053c240`, `FUN_0053c4b0`, `FUN_0053f150`, `FUN_0053efa0`,
`FUN_0053c810`, `FUN_0053cbc0`, `FUN_0053ca50`, `FUN_0053ca30`,
`FUN_0051fb70`, `FUN_0051fb80`, `FUN_0051fbb0`, `FUN_005201c0`,
`FUN_005202d0`, `FUN_004f5440`, `FUN_0053d020`, `FUN_0053d0e0`,
`FUN_0053d100`, `FUN_0053d3c0`, `FUN_0053d430`, `FUN_00553aa0`,
`FUN_00553b80`, `FUN_00504e60`, `FUN_005535b0`, `FUN_00578ab0`,
`FUN_00578c00`, `FUN_00578d00`, `FUN_00578f30`, `FUN_005152e0`,
`FUN_00515390`, `FUN_00515440`, `FUN_005555e0`, `FUN_00555920`,
`FUN_00556390`, `FUN_005542f0`, `FUN_004fdc70`, `FUN_004f6800`,
`FUN_004ff8e0`, `FUN_004fdb00`, `FUN_004fe710`, `FUN_0049a1b0`,
`FUN_0049a350`, `FUN_0049a8b0`, `FUN_0053c2e0`, `FUN_0044f060`,
`FUN_0044f180`, `FUN_0044f5e0`, `FUN_0044f640`, `FUN_0060b840`,
`FUN_0060b9d0`, `FUN_0042db70`, `FUN_0041db60`, `FUN_0041e390`,
`FUN_0041e500`, `FUN_00441190`, `FUN_0045aac0`, `FUN_00429ce0`,
`FUN_004591d0`, `FUN_004a2630`, `FUN_0049ee20`, `FUN_00452fc0`,
`FUN_004a7790`, `FUN_0049f130`, `FUN_004ac120`, `FUN_00606380`,
`FUN_0045c660`, `FUN_0045c6b0`, `FUN_0045c830`, `FUN_00458b80`,
`FUN_004aa470`, `FUN_004a3130`, `FUN_004aa380`, `FUN_004a09a0`,
`FUN_00555460`, `FUN_00551100`, `FUN_00551270`, `FUN_00552e80`,
`FUN_00552ff0`, `FUN_005531b0`, `FUN_00552d10`.
