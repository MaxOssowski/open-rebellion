# Icon menus after the pointer leaves, and what greys an icon's Move

Date: 2026-11-12. Cell CMD-03-C018 (quadrant icon right-click menu).
This is a trace only. No code, no WASM build, no gate, no A0 capture.
The ledger JSON is not edited.

## What was asked

The ledger note ends: "Still missing: other icons' menus after leaving,
and the Move-greying cause." So two questions:

1. A0 (`2026-11-11-icon-menu-topic/00`, `07`, `12`) shows the defenses
   icon's last lit row staying lit after the pointer leaves the menu.
   Do the facility, fleet and missions icons' menus do the same?
2. What greys a defenses icon's Move and Confirmed Move? A0 showed
   Coruscant's icon grey on days 12 to 17 with every visible member free.

## The trace

### 1. One menu window for every icon

- A right release over a sector window goes to `FUN_004ac5c0`
  (`Class_0065bf80_vt1c`). It copies the selection, lists its orders
  through `FUN_0051d990`, makes each order an item with `FUN_00442590`,
  and opens the menu with `FUN_00442380`. Nothing in it reads the
  icon's kind.
- The kind (`4`, `8`, `0x10`, `0x40`) reaches only the order list:
  `FUN_00507290` -> `FUN_0050f2e0(kind & ~1)` (`sector-icon-menus.md`).
- `FUN_00442380` -> `FUN_00442860` builds the window with
  `FUN_004422f0` -> `FUN_004aa960`. That sets vtable `0x0065bef0`
  ("Game Menu Window", string at `0x006a8718`). Its procedure is
  `FUN_004aab50` (`Class_0065bef0_vt14`). A submenu window
  (`FUN_00442860`'s recursion, `FUN_004ab3a0`) has the same class.
- `FUN_004aab50`:
  - `WM_MOUSEMOVE` (`0x200`) tests only enabled items (`+0x58 != 0`).
    The one under the pointer becomes `+0x118` (`param_1[0x46]`). If the
    pointer is in the window but off that item, `+0x118` is cleared.
  - `WM_PAINT` (`0xf`) draws `+0x118` with `FUN_004aba60(.., 1)`, the
    highlight, wherever the pointer is.
  - `WM_SHOWWINDOW` (`0x18`) with hide clears `+0x118`.
  - `WM_TIMER` (`0x113`) shows the lit item's submenu, if it has one
    (`+0x3c`).
- `FUN_004aba60` draws an item disabled, normal or lit. Separately, it
  draws the check bitmap (`+0x5c`, `+0x1c`) in every state. So a lit
  Reserved row keeps its check mark (STRATEGY 11902).

**Result.** Every icon's menu is the same window with the same
procedure. The row last lit stays lit after the pointer leaves, for the
facility, fleet and missions icons too. The code settles this. A0 `00`,
`07` and `12` show the shared procedure at work.

One case only the fleet icon has: its Planetary Bombardment parent. If
that row is the one left lit and it is enabled, the 500 ms timer still
opens its submenu after the pointer has left. A0 `menus/08`, `09` show
the parent grey at the player's own system, so this was never seen.

### 2. What greys the defenses icon's Move

The enabled check is `FUN_0053c100`: `FUN_0053c4b0` builds the
sub-orders, then each sub-order's `+0x18` must pass
(`sector-icon-menus.md`, "The defenses icon's Move").

**The team.** `FUN_00512700` kind 8 walks the system's direct children
in mode 3 (`+0x50` bit 6, existing). The walk never goes into fleets or
missions (`object-state-flags.md`). Existing is created and not
destroyed. It does not need completed, and it does not exclude en route.

**The split.** `FUN_0053cbc0` puts only completed characters and special
forces into the `0x241`. Everything else goes into the `0x204`,
uncompleted special forces too.

**The `0x241`** (`FUN_0051fe20` -> `FUN_00553770` -> command `0x250`
`+0x18` -> `FUN_0054c110`, and `FUN_0054bf00`). It refuses a member
that is not the side's, travelling, or on a mission. It also refuses
members at different places (`FUN_0054bf00`, `0x40`/3). At an icon all
members are direct children of one system, so they share one place.

**The `0x204`** (`FUN_0051fe20`):
- `+0x4c` (`FUN_005201c0`) fills a context with `FUN_0051fcb0`:
  `[0]` the order's side (`+0x20`), `[2]` the target, `[3]` the order's
  kind from slot `+0xc`. For the `0x204` (`FUN_0053d0e0`) that kind is
  `0x204`.
- `FUN_005535b0` makes one move command per member (`FUN_0053d3c0` ->
  `Event201_Init`, vtable `0x00669558`). `FUN_0054f3e0` ->
  `FUN_00541eb0` copies the context into the command.
- `FUN_00553770` runs each command's `+0x18`, `FUN_00578c00`
  (`Event201_vt18`). That calls the object's slot `+0x6c` with the
  context, then `FUN_005152e0`.
- Slot `+0x6c`, read from REBEXE.EXE:
  - regiment, vtable `0x0065e438`: `0x00504350`;
  - squadron, vtable `0x0065def0` (`FightSquad`): `0x005576a0`;
  - capital ship, vtable `0x0065dc10`: `0x005576a0`.
  Both functions call `FUN_004f9860(context, status)`.
- `FUN_004f9860` refuses with these codes:
  - 1: the object's side (`+0x24` bits 6..7) is not the context's side;
  - 2: not completed (`+0x50` bit 2), except when the context's kind is
    `0x203`, `0x215`, `0x204` or `0x241`;
  - 3: destroyed (`+0x50` bit 3);
  - 4: en route (`+0x50` bit 4).
  The decompile shows the side and kind read through `piStack_8`. The
  callers pass `(context, status)`, so this reads it as the context.
- `FUN_005152e0` -> `FUN_005555e0` refuses only families `0x20..0x21`
  in a blockade (`regiment-unload.md`). It does not apply here.

**So the icon's Move is grey when any of these holds:**
- the side has nothing at the system (`0x16`);
- a completed character or special force of the side there is
  travelling or on a mission;
- a regiment, squadron or unbuilt special force of the side there is en
  route (bit 4).

**And it is not greyed by a unit still being built.** A product exists
from its order, inside its destination system (`FUN_0052a140`,
`planet-squadrons.md`; `ship-deployment-fleet.md`). It is in the
mode-3 team, but kind `0x204` skips code 2, and it is not en route
before completion. En route comes from bit 5, which `FUN_0052bee0` sets
only at completion, when the product's place differs from the
facility's (`build-delivery.md`).

**What this says about Coruscant, days 12 to 17.** The cause must be an
object of the Empire at Coruscant that is en route, or a member on a
mission or travelling. A completed delivery on its way there fits: it
is a child of Coruscant with bit 4 set from completion to arrival
(`FUN_0052bee0`, `FUN_004f8240`). The code settles which states grey
the Move. It does not show which object greyed Coruscant: no capture
shows the Empire's deliveries on those days.

Not read: the object slot `+0x58` that `FUN_004f9860` calls first, and
a special force's slot `+0x6c`.

## The port

**Menus after leaving.** Every icon's menu is `MenuObject::SystemIcon`.
`object_menu::draw_object_menu` draws it with
`game_menu::draw_game_menu_with_submenu`, the same function for all four
kinds. Since b194 that function keeps the lit row when the pointer is
outside the window. Inside the window it lights only the enabled row
under the pointer, and clears the highlight off every row. That matches
`FUN_004aab50`. The check mark is drawn whether or not the row is lit,
as in `FUN_004aba60`. The fleet icon's submenu parent stays lit, and its
timer still fires, while the pointer is outside. That matches
`WM_TIMER`.

**The Move rule.** `defenses_move_enabled` (`rebellion-app/src/main.rs`)
gives:
- an empty team: grey;
- characters and special forces: `MissionState::move_order_enabled`
  (`FUN_0054c110`);
- a regiment or squadron flying in:
  `TroopTransportState::surface_arrival_pending`;
- a product on its way: `DeliveryState::surface_arrival_pending`
  (regiments, fighter squadrons, special forces);
- each regiment and squadron class: its own move check.

The port makes a product only at completion. So a queued product is in
no team and greys nothing. That matches the trace above.

No difference was found.

## Verdict

**Still partial, narrowly. No port bug.**
- Menus after leaving: settled by the code. All four icons share one
  menu window and one procedure, and the port shares one function. This
  part is a pass candidate. An A0 frame is not needed for it, but one
  cheap capture would confirm it (below).
- Move greying: the rule is settled by the code, and the port follows
  it. Coruscant's grey on days 12 to 17 is still inferred to be a
  delivery. No A0 shows it.

## A0 needed to close the cell

Windows 10 VM, a running Empire game. Each step is one QMP screenshot.
1. At a facility system, order one TIE squadron with Destination a held
   system X with one free regiment. While it is queued, right-click X's
   defenses icon. Expect Move white (code 2 skipped for `0x204`).
2. Run until it completes and departs. Right-click X's defenses icon
   again. Expect Move, Confirmed Move and Scrap grey (code 4).
3. After it arrives, right-click again. Expect white.
4. Right-click a fleet icon. Pass the pointer over Move, then leave the
   menu. Expect Move still lit.

Steps 1 to 3 would show the delivery case and the queued case. Step 4
would confirm the shared highlight on a second icon.
