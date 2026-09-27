# Build delivery: en-route manufactured objects

Recovered 2026-09-27 from REBEXE.EXE with Homebrew Ghidra 12.1.3 (read-only,
no analysis). Finding F-030 in the full-functionality audit. The `+0x50` state
bits are in `object-state-flags.md`.

## State bits used

The fleet view vtable `0x0065d438` slots `+0x178 + 4 * bit` name the rest of
the `+0x50` bits (setters `FUN_004f7720`..`FUN_004f7a30`, `FUN_004f7aa0`):

| Bit | Mask | Name |
|-----|------|------|
| 7 | `0x80` | `GameObjObservedByAlliance` |
| 8 | `0x100` | `GameObjObservedByEmpire` |
| 9 | `0x200` | `GameObjDamagedNotif` |
| 10 | `0x400` | `GameObjHyperdriveActiveNotif` |
| 11 | `0x800` | `GameObjAutoroutingNotif` (`FUN_004f7870`) |
| 12 | `0x1000` | `GameObjAutoscrapRequestNotif` (`FUN_004f78e0`) |
| 13 | `0x2000` | `GameObjLockedNotif` |
| 14 | `0x4000` | `GameObjReadyForDeleteNotif` |
| 15 | `0x8000` | `GameObjConstructedNotif` |
| 16 | `0x10000` | `GameObjDeployedNotif` (`FUN_004f7aa0`, slot `+0x1b8` `FUN_004fc570`) |

En route (bit 4) is recomputed by `FUN_004f8240`:

```c
enroute = FUN_005406d0(obj)            // any object but the galaxy (type 0xf1)
       && deployed(bit 16) && !destroyed(bit 3)
       && (enroute_active(bit 5) || container_of(obj).enroute(bit 4));
```

So an object is en route while it travels itself, or while its container
(`FUN_004f6b50`) is en route, such as a ship in a travelling fleet.

## Build order and product

- The manufacturing manager (the facility-side class with the tick at vtable
  slot `+0x208`, `FUN_0052b960`; vtables near `0x0065fbf8` and `0x006629a0`,
  constructors `FUN_005279d0` and `FUN_0055b2f0`) keeps a queue of product
  objects in the list at `+0x54 -> +0x18`. The queued units are real game
  objects from the moment they are ordered.
- `FUN_0052aac0` chooses the current product: the first queued object that
  `FUN_00528890` accepts. `FUN_00529770` stores its key at manager `+0x74`
  (DeploymentKey, notifier `FUN_0052bd00` `ManuMgrDeploymentKeyNotif`, event
  `0x321`).
- Objects carry `DestinationLocationAtDeparture` (key at `+0x3c`, setter
  `FUN_004f7120`, notifier `FUN_004fbe50`) and `DestinationCount` (arrival tick
  at `+0x44`, setter `FUN_004f7390`, notifier `FUN_004fbf10`). The
  `capshp.cpp` validators `FUN_004f7260`/`FUN_004f72f0` guard
  `DeploymentCount` and `DestinationCount`.

## Completion (`FUN_0052b960`)

```c
if (current_product(mgr + 0x1c) && mgr[+0x5c] != 0 && mgr[+0x5c] >= mgr[+0x68]) {
    product = lookup(mgr + 0x74);                 // FUN_0052be30 -> FUN_0053efa0
    if (product) FUN_0052bee0(mgr, product, ctx);
}

FUN_0052bee0(mgr, product, ctx):
    a = mgr->slot_0c();                            // the manager's location key
    b = product->slot_0c();                        // the product's location key
    set_deployed(product, 1);                      // bit 16, FUN_004f7aa0
    if (a != b && a && b) set_enroute_active(product, 1);   // bit 5, FUN_004f7640
    set_completed(product, 1);                     // bit 2, FUN_004f74f0
```

Progress `+0x5c` reaching the cost `+0x68` completes the product. A product
whose location differs from the manufacturing facility's starts travelling.

## Travel

Any change of container goes through `FUN_00514a60(obj, from, to, ctx)`, which
calls `FUN_00556430` for a mobile object (slot `+0x38`):

```c
FUN_00556430(obj, from, to, ctx):
    if (!obj->slot_38() || autorouting(bit 11)) return;
    if (system_of(from) != system_of(to)) {        // FUN_00555540 on both
        FUN_00555d30(obj, from, to, &ticks);
        set_enroute_active(obj, 1);                // bit 5
        schedule(0x387, ticks, obj);               // FUN_004f8010 -> FUN_0053fab0
        obj[+0x3c] = key(to);                      // FUN_004f7120
        set_autorouting(obj, 1);                   // bit 11, FUN_004f7870
        obj[+0x44] = clock() + ticks;              // FUN_004fd340 (callback DAT_006b2b08), FUN_004f7390
    }

FUN_00555d30(obj, from, to, &ticks) -> FUN_00555b30(obj->slot_34(), 1, side, from, to, &ticks)
FUN_00555b30: ticks = 0 unless both ends are systems (types 0x90..0x98) and differ;
              then ticks = FUN_0055d8c0(coords(from), coords(to), speed)   // FUN_00509620
FUN_0055d8c0(a, b, speed):
    d = isqrt((b.x - a.x)^2 + (b.y - a.y)^2);      // FUN_0055d860, FUN_0053e1d0
    if (d == 0) return 0;
    t = (d / GNPRTB[5120]) * speed / 100;          // DAT_006bb6e8, FUN_0053e190
    return t == 0 ? 1 : t;
```

GNPRTB 5120 (`0x1400`, loaded by `FUN_0055cb60`) is 5 in every column of the
shipped GNPRTB.DAT. The travelling object is represented by itself: bits 5 and
11, the destination key at `+0x3c`, the arrival tick at `+0x44`, and a pending
galaxy event `0x387`. There is no hidden fleet.

Correction: `ghidra/notes/bombardment.md`, `COMBAT-SUMMARY.md`,
`modders-taxonomy.md` and `crates/rebellion-core/src/bombardment.rs` read
`FUN_00556430`, `FUN_00555d30`, `FUN_00555b30`, `FUN_0055d8c0` and
`FUN_00509620` as bombardment. They are this transit-time chain: the "power"
shorts are system coordinates and the "damage" is travel ticks.

## Arrival

- Event `0x387` is registered by `FUN_0051ef80` (`FUN_0054f140(0x387,
  FUN_0057be20)`); its class (`FUN_0057bd30`, vtable `0x00669838`) validates
  the target key at `+0x3c` in `FUN_00586150`.
- `FUN_004fb520`, an object vtable slot shared by many classes, clears en
  route active (bit 5), autorouting (bit 11) and autoscrap request (bit 12),
  then recomputes en route (`FUN_004f8240`) and the derived bits.
- `FUN_004fba10` cancels an object's pending `0x387` (`FUN_0053fa60`).
- `FUN_004fc080` raises `GameObjDestroyedOnArrivalNotif` (event `0x303`) for a
  destroyed object that was en route active (bit 5) with a non-zero
  `+0x40 & 0xff0000`.

## Open

- Where a product sits while queued: `FUN_0052bee0` implies it already has a
  location different from the facility's when it is sent elsewhere, but the
  order path that creates it and moves it (and so whether the travel clock
  starts at the order or at completion) is not traced. `FUN_00528890`'s
  acceptance rule is not read.
- The speed slot `+0x34` and the mobile slot `+0x38` per class (ship,
  fighter, regiment, facility) are not read.
- The arrival handler's placement step (which container the object joins,
  fleet or system list) is not traced past `FUN_004fb520`; `FUN_00556390`
  (via `FUN_00515440`) handles an object dropped into another system's
  container without autorouting.
- The `+0x40 & 0xff0000` field in the destroyed-on-arrival test is unnamed.
- `FUN_004fd340` returns the value of the callback at `DAT_006b2b08`,
  read here as the game clock because it is added to a tick delay; the
  callback itself is not traced.
- The player's destination choice and the en-route display were not looked
  up.
