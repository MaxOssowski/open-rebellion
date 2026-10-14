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

- Scrap for ships, fighter squadrons, fleets and regiments aboard a fleet
  (`FUN_005575f0`, `FUN_004fdbc0` → `FUN_004ffa70`).
- The player's menu item: `object_menu.rs` still draws Scrap disabled.
- Reasons `0x15` and `0x16` do not refund yet (autoscrap, queue scrap).
- The autorouting of characters in a scrapped fleet (`FUN_004ef2e0`).
