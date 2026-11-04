# Moving the Alliance headquarters (read 2026-11-04)

The manual (p. 134, "Moving Alliance Headquarters", Fig. 3.82) says the HQ
icon can be dragged onto a new system, or right-clicked for Move and
Confirmed Move. `headquarters-overlay.md` covers the overlay and its menu.

## The orders

`FUN_00526c00` lists `0x201`, `0x202` and `0x204` for the HQ (vtable
`0x0065f280`). Move and Confirmed Move follow `move-order.md`:
- `FUN_0053c4b0` expands the order. The HQ is no character or special
  force, so it gets one per-object move `0x204` (`FUN_0053ca50`).
- `0x204` builds the move command `0x201` (`FUN_00578ab0`, vtable
  `0x00669558`).

## Can it move (`+0x18`, `FUN_00578c00`)

- **Object check.** The HQ's slot `+0x6c` is `FUN_005269b0`, which runs
  `FUN_004f9860`. Status 1 means not the ordering side's (`+0x24` bits
  6..7). Status 4 means en route (`+0x50` bit 4).
- **Blockade.** `FUN_005555e0` covers ALLFACSD objects (`0x20..0x21`). It
  refuses `0x90`/4 when two things hold:
  - the object's system has its blockade bit (`+0x88` `0x20`);
  - `FUN_0055a020(ships, fighters)` comes to less than 31. The ships and
    fighters are counted over the system's usable fleets (`FUN_004ffe70(system,
    1)`, each fleet's ships and their squadrons) and the planet's squadrons
    (`FUN_005039d0`). The value is `100 - ships * GNPRTB[7684] - fighters *
    GNPRTB[7685]`, clamped at 0.

## The destination

- **Side.** `FUN_0053d430` refuses `1`/`0x28` when the destination's side
  bits differ from the order's. The HQ is no fleet, capital ship or
  regiment, so no exception applies. The HQ therefore moves only to its
  side's systems.
- **Speed.** The HQ's slot `+0x34` is `GetGnprtb_1` (`FUN_004f63f0`): GNPRTB
  1 for an existing object (`+0x50` bit `0x40`). It is never 0, so
  `FUN_00555920`'s `1`/`0x18` never applies.

## Execute (`FUN_00578f30` → `FUN_00556390`)

- **Across systems.** The HQ changes container at once (slot `+0xa8`), with
  the in-transit bit set (`FUN_004f7640`, `+0x50` bit 5). Its derived en
  route bit 4 then holds until arrival (`build-delivery.md`).
- **Within one system.** There is no transit.

## Support

A system's content change (`System_vtc8`, `FUN_00508660`, slot `+0xc8` of
vtable `0x0065e638`) handles an ALLFACSD object (`0x20..0x21`) with
`GetGnprtb_7706` (`FUN_0055a0f0`):
- **Amount.** It is GNPRTB 7706 when the object enters (third argument
  set), and its negative when it leaves. Shipped, GNPRTB 7706 = 5 at every
  level.
- **Who it applies to.** `Event373_Handle` (`FUN_0050c9f0`) shifts side 1's
  support by that amount, and only on a populated system (`+0x88` bit 0).
  The shift goes through `FUN_00559be0`, so it is divided by GNPRTB 7681
  near a Death Star when it favours the Alliance. It is then clamped by
  `FUN_0053e0d0`.

So a move costs the origin 5 points of Alliance support and gives the
destination 5. The manual (p. 134) names only the drop: "a small drop in
popular support at the system in which it was originally located". An HQ's
loss of existence is a separate path: event `0x300` → `FUN_00561470` →
`FUN_00561780`, the character loyalty shift. A move does not take that path.

## What reads the HQ en route

- **The overlay.** `FUN_0045bbb0` finds the HQ with `FUN_00526cf0(record,
  3)`, mode 3 "existing" (`object-state-flags.md`). It reads the viewing
  side's record of the system (`FUN_004f3220`), and only when a system
  refresh calls it (`FUN_0045b770`). The window keeps one HQ overlay, id
  `0x10000`. A refresh adds it when the HQ bit is set, and removes it
  when the bit is clear (`FUN_0060a860(0x10000)`). A0
  headquarters-move/07 and /09 show an open window that still drew 904
  over Pzob after the HQ stood at Gandolo IV, until the window was
  reopened. hyp: the move sends no refresh to either system.
- **Victory.** `FUN_00506110` finds the HQ by family through
  `FUN_0053efb0`, wherever it stands.
- **Status.** `FUN_00444e20` leads with `FUN_004fd2b0`'s ETA rows (34662
  "ETA Destination:", 14356 "Day "). `FUN_0044a470` writes 34628
  "Enroute" for `+0x50` bit 4, 34659 "Active" for a completed object, and
  34647 "Under Construction" otherwise.
- **Confirmation.** `FUN_00487cc0` asks for `0x202`. It also asks for a
  `0x201` whose object stands in a blockaded system of its side.
  `FUN_0049a350` then shows 0x7056 and pictures 1031/1030, because the HQ is
  no fleet.

## Ported (2026-11-04)

- **The core.** `rebellion_core::headquarters_move` holds three functions:
  - `move_enabled` is the rows' `+0x18`;
  - `plan` gives the refusals and the trip's days at the default speed;
  - `move_headquarters` moves the facility key and the `is_headquarters`
    flag at once, and points `VictoryState::alliance_hq` at the
    destination. It also shifts the Alliance's support by -7706 at a
    populated origin and +7706 at a populated destination
    (`uprising::apply_support_change`).
- **The trip.** It is kept in `TroopTransportState::headquarters`, save
  v112, and `advance` ends it on the arrival day.
- **The window and drag.** `object_menu.rs` enables the rows through
  `OrderGates::headquarters_move`. A sector-window drag of 904 drops
  `MenuObject::Headquarters`, which issues the same order, as a Confirmed
  Move with Ctrl held.
- **The order.** `main.rs` `order_headquarters_move` asks or moves as
  `FUN_00487cc0` does. The checkmark sends
  `MoveConfirmationAction::MoveHeadquarters`.
- **Status.** `status_rows.rs` shows the ETA and "Enroute" while the HQ is
  en route.
- **port notes:**
  - A Fleet window's fleet as the destination is refused. The original's
    route (`FUN_005531b0`) would accept a fleet, and its leg is untraced.
  - A destroyed system counts as another side's.
  - No arrival message is traced.
  - Rules that read only usable objects (mode 1) still see the HQ en
    route.
  - The sector window redraws 904 at once, where A0 keeps an open
    window's mark until it is reopened.
