# The Scrap order (`0x200`)

Read 2026-10-14. The manual (p. 86, "Scrapping Facilities") says scrapping
returns the maintenance and some of the refined material the unit used,
frees energy on the system, and works on "any facility, troop, or ship"
from its right-click menu. The AI's modules 1, 3, 6, 7 and `0xa` issue it
(`ai-production-modules.md`).

## The order

`FUN_0051f4b0` registers `0x200` to the factory `FUN_0053dd20`, which
allocates 0x44 bytes and calls `FUN_0053dcd0`: the order base
`FUN_0051fa20`, vtable `0x006619b0`. Like Destination (`0x214`,
`move-order.md`), it extends the order base directly and has no sub-order
list.

| Slot | Function | Role |
|---|---|---|
| `+0xc` | `FUN_0048b450` | Kind `0x200` |
| `+0x18` | `FUN_0051fe20` | Enabled: built, then `+0x4c`, then the commands' `+0x18` (`FUN_00553770`) |
| `+0x1c` | `FUN_0051ff30` | Validator: enabled, then `+0x50`, then the commands' `+0x1c` (`FUN_00553960`) |
| `+0x20` | `FUN_00520040` | Execute: validate, then submit the commands (`FUN_005539f0`) |
| `+0x48` | `FUN_0053dda0` | Create a command (0x50 bytes, `FUN_00579570`, vtable `0x006695a8`) |
| `+0x4c` | `FUN_005201c0` | One command per team member (`FUN_005535b0` over `FUN_0053f150`) |
| `+0x50` | `FUN_005202d0` | Built: `+0x1c` non-zero, else `1`/`1` |

There is no destination check: `+0x50` is the base's.

## The command (vtable `0x006695a8`)

Each command holds its object at `+0x3c`:

- `+0x18` `FUN_005796c0`: resolve the object (`FUN_00504e60`, `1`/`0x12`
  when it does not resolve), then the object's slot `+0x5c`.
- `+0x1c` `FUN_00579790`: the same, then the object's slot `+0x60`.
- `+0x20` `FUN_00579860`: resolve the object, then call its slot `+0x98`.

## The object's slots

| Class | `+0x5c` | `+0x60` | `+0x98` |
|---|---|---|---|
| Facilities `0x22..0x25`, `0x28..0x2a`, `0x2c`, `0x2d` | `FUN_004f28d0` | `FUN_004f6620` | `FUN_004f84e0` |
| Headquarters `0x20` (`0x0065f280`) | `FUN_00526940` | `FUN_004f6620` | `FUN_004f84e0` |
| Fleet (`0x0065d438`) | `FUN_004fdbc0` | `FUN_004f6620` | `FUN_004f84e0` |
| Capital ship (`0x0065d650`) | `FUN_005575f0` | `FUN_004f6620` | `FUN_004f84e0` |
| Regiment (`0x0065e438`) | `FUN_005042a0` | `FUN_004f6620` | `FUN_004f84e0` |

- `+0x5c`, the check: for facilities, capital ships and regiments it is
  the object check `FUN_004f9860` (`object-popup-menu.md`). It refuses
  the following:
  - an object whose class does not list the order (slot `+0x58`:
    `FUN_00539c20` for facilities, the base list and `0x200`;
    `FUN_00504b30` for regiments);
  - another side's object, status 1;
  - one not completed (`+0x50` bit 2), status 2;
  - one destroyed (bit 3), status 3;
  - one en route (bit 4), status 4.

  Any refusal ends as `1`/`1`. The fleet's check is `FUN_004ffa70`; it is
  not read here. The headquarters' `FUN_00526940` always refuses with
  `1`/`0x17`.
- `+0x60` `FUN_004f6620` calls `+0x5c` again.
- `+0x98` `FUN_004f84e0` runs `+0x60`, then `+0xa0`. That is
  `FUN_00534c00` in every class above: slot `+0xac(0x14, ctx)`.
- `+0xac` is `FUN_004f8660` → `FUN_004f71d0`. It writes the destroyed
  reason byte `+0x40` (here `0x14`) and calls slot `+0x100`, which is
  `FUN_004f2ad0` → `FUN_004fb200` → `FUN_004f7560` in these classes. That
  sets the destroyed bit `+0x50` bit 3 and raises `GameObjDestroyedNotif`
  (`object-state-flags.md`).

## The refund

`FUN_005140c0`, the side's handler for the destroyed object, calls
`FUN_00530270` once the destroyed bit is set. `FUN_00530270` adds
`FUN_004f2980() / 2` to the side's refined `+0x7c` (setter
`FUN_0052eff0`) when two things hold:

- the reason byte `+0x40` is `0x14..0x16`;
- the object is completed (`+0x50` bit 2).

`FUN_004f2980` returns class `+0x48` (`FUN_0053b860`), the refined
material cost, or 0 without a class. So Scrap (`0x14`) refunds half the
cost. Reason `0x15` is Autoscrap (`entity-system.md`); reason `0x16` is
the cause the manufacturing manager's queue scrap uses
(`manufacturing-manager.md`). The maintenance comes back because the
object no longer counts.

`FUN_004ef2e0`: a character whose container was scrapped (`+0x40 ==
0x14`) starts autorouting rather than drifting.

## Ported (`crates/rebellion-core/src/scrap.rs`)

- `check`: the object resolves (`0x12`), is not a headquarters (`0x17`),
  and is the ordering side's (1).
- `scrap`: removes the facility or regiment and adds half its class's
  refined cost to the owner's stock (`ProductionState`).
- `AIAction::Scrap`, applied by the headless integrator and the
  interactive loop.

port:
- Every object in the port's world is completed and not destroyed, and a
  regiment on a planet is never en route.
- The refund reaches waiting yards on their next daily pass; the
  original's setter notifies them at once (slot `+0x1d4`).

## Open

- Scrap for regiments aboard a fleet on their own. A squadron aboard a
  fleet is ported (2026-11-13, `ScrapTarget::FleetFighter`, the same
  `FUN_005575f0` as on a planet; A0 `2026-11-13-drag/16`: Scrap white on
  three of a fleet's squadrons).
- Reason `0x15` refunds, in `autoscrap.rs` (2026-10-17).

## Reason `0x16` never refunds a queued product (2026-11-13)

The manufacturing manager's queue scrap (`FUN_0052a2c0`, from Build's
slot `+0x1f0` `FUN_00529ab0`, Stop's `+0x1f8` and a yardless manager)
destroys each queued product with reason `0x16`. `FUN_00530270` refunds
only a completed object (`+0x50` bit 2), and a queued product is created
with completed = 0 (`FUN_0052a140` → `FUN_004f7d50(.., 1, 0)`) and stays
so until it completes, when it leaves the queue. So `0x16` refunds
nothing in practice. The port's `ManufacturingState::stop` and
`build_units` drop the queue without a refund
(`production_cycle::tests::replacing_an_order_in_progress_refunds_no_refined_material`).
- The autorouting of characters in a scrapped fleet (`FUN_004ef2e0`):
  the port sets them adrift at the fleet's system instead.

## The player's order (2026-11-01)

`FUN_00487cc0` returns 1 for kind `0x200` with no condition, so a Scrap from
the menu always asks. `FUN_0049a350` case `0x200` fills the confirmation:

- the text: RCDATA 0x7050 "Are you sure you want to scrap the following
  units?", then `FUN_0049a880` appends 0x7054 ("\n{1:1}") with each
  member's name (`+0x30`);
- the picture `+0x2e`: `0x408` (1032) for side 1, `0x409` (1033) otherwise;
- `+0x30`: `0x461` / `0x462`, as a Retire's.

The window is the move confirmation's (`move-order.md`); its checkmark
resubmits with force 1.

A0 (`.artifacts/a0-vm/status/`) shows the row enabled on the player's own
regiment (11), planet squadron (20), battery (22) and training facility (28),
and disabled on an enemy ship (02) and fleet (05). Its place follows the sort
key: Scrap (2000) after Encyclopedia and Status.

Ported: `MenuObject::scrap_target`, `OrderGates::scrap` (`scrap::check`),
`ConfirmedOrder::Scrap` and `scrap::target_name`; the checkmark runs
`scrap_team`. `fleet-window.mjs` "scrap" checks it on a regiment in the
browser.

port: a fleet, a squadron aboard a fleet and a sector window icon still
draw Scrap disabled (`FUN_004fdbc0`, `FUN_005575f0`, `FUN_00512700` are not
ported).

## A fleet (2026-11-01)

The fleet's check `FUN_004fdbc0` → `FUN_004ffa70`: the object check
`FUN_004f9860` (own side; status 4 en route), then each ship must be
completed (else `0x10`) and not en route (else `0x11`). A0 greys an
arriving fleet's Scrap (`.artifacts/a0-vm/menus/22-fleet4-menu.png`).

Destroyed with reason `0x14`, the fleet hands the reason to every child
(`FUN_004fb520`, `FUN_004f8710`), so each ship, squadron and regiment
aboard is scrapped and refunds half its refined cost. A0, day 318 (evidence
`2026-11-01-fleet-scrap-a0`): Fleet 1 (corvette 14, transport 10, two
regiments 8) returned 20 refined and freed 47 maintenance the same day; the
confirmation lists only "Fleet 1".

Ported: `ScrapTarget::Fleet`, `scrap::check_with_movement` (the en-route
refusal), and the refund and removal in `scrap::scrap`, which now takes the
troop transport state for the cargo. The characters aboard go adrift
(port, for the autorouting) and the special forces land at the system.
