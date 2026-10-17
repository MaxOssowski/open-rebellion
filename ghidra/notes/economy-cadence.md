# The economy's cadence: what changes a system, and when

Read 2026-10-17 for F-037. Every tick the port runs `EconomySystem`,
which rebuilds each system (until 2026-10-17 it also drifted its
support). The original has no such loop:
- `FUN_00508250` (system slot `+0xc0`) runs once, in the setup stage
  `0x17` (`timer-scheduler.md`).
- After that a system changes only through its field hooks (when an input
  changes) and its timers.

## The derived fields and who computes them

Each of these setters stores a field and notifies its views and its master
slot (`ghidra/generated/setters.tsv`).

| Field | Setter | Computed by | Value | Port |
|---|---|---|---|---|
| `+0x6c` SmugglingPercent (-100..100) | `FUN_00509ec0` | `FUN_0050b230` → `FUN_00559c40` | see below | step 1, ported 2026-10-17 (it had been misread as a support drift) |
| `+0x70` ProductionModifier | `FUN_00509f40` | `FUN_0050b2c0` → `FUN_00559c10` | GNPRTB `DAT_006bb44c` × 100 / support (1 for 0); 100 when neutral | "collection rate" (step 2) |
| `+0x80` TroopRegRequired | `FUN_0050a290` | `FUN_0050b5a0` → `FUN_00559fe0` | `FUN_0055a050(support)`, ÷ `DAT_006bb3dc` for an Empire system with `+0x88` bit 11, × `DAT_006bb3e4` in uprising (bit 2) | garrison requirement (step 3) |
| `+0x84` holder | `FUN_0050a300` | `FUN_0050b610` | `system-control.md` | step 5, ported 2026-10-17 |
| the withdraw percent | | `FUN_0050b310` | `blockade-troop-withdrawal.md` | step 4 |

`FUN_00559c40(side, support, bit 11, fleets, fighters, troops, own fleet)`
is the smuggling percent:
- with no own fleet and support at most `DAT_006bb478`, a base by support
  band (`DAT_006bb424`, `DAT_006bb3f8`, `DAT_006bb4b0`) less the weighted
  forces, clamped;
- negative for the Empire;
- else 0.

The port's `calculate_support_drift` read exactly this function, but
applied the result to the support every tick. The original stores it as
SmugglingPercent and never changes the support with it, so that support
drift had no source.

`FUN_00509ec0` rejects a value outside -100..100 (`FUN_0053fc90`), and
stores and notifies (`+0x1e4`, `SystemSmugglingPercentNotif`) only when the
value changes.

### Port (2026-10-17)

- `economy::smuggling_formula` is `FUN_00559c40`, and
  `economy::smuggling_percent` is `FUN_0050b230`. The inputs are the
  holder's alive capital ships in its fleets (`FUN_00509090`), its
  squadrons on the planet only (`FUN_00508fb0`), its regiments
  (`FUN_00509020`) and the Death Stars in its fleets (`FUN_00509140`, CAPSHPSD
  family `0x18`).
- Step 1 stores the result in `SystemEconomy::smuggling_percent` (save
  v61) and emits `SmugglingPercentChanged` on a change. Nothing moves the
  support with it.
- port: the economy loop still skips unpopulated systems, so a system held
  only by regiments keeps 0.
- port: bit 11 of `+0x88` is still the port's `strong_support` guess.

## The hooks

They run when a system's inputs change. Who calls each one is still open
(`timer-scheduler.md`).

- `FUN_005109f0`, the regiments changing (slot `+0x1d0`): `FUN_0050b610`
  (holder), `FUN_0050b5a0` (TroopRegRequired), `FUN_0050b230`
  (SmugglingPercent) and `FUN_0050b2c0` (ProductionModifier). While the
  system is blockaded (`+0x88` bit 5) it arms timer `0x386`.
- `FUN_00510820`, through `FUN_004fae40`: `FUN_0050ac80`, `FUN_0050b890`,
  `FUN_0050c350`, `FUN_0050a4a0(0)` (clears the uprising bit),
  `FUN_00509ec0(0)`, `FUN_0050b230`, `FUN_0050b5a0`, `FUN_0050c820`
  (arms the informant timer `0x38e`) and `FUN_0050ac10(1)`.
- `FUN_00511740`: `FUN_0050b5a0` and `FUN_0050b230`. It is probably the
  support's hook, since both depend on support.
- `FUN_00511300`, the blockade changing: `FUN_0050c0b0`, `FUN_0050bbb0`,
  `FUN_0050c180`, `FUN_0050c1b0`, `FUN_0050c1e0` and `FUN_0050b310` (the
  withdraw percent), then `FUN_0050ce20` for both sides. It arms timer
  `0x386`.

## The timers that change support

All go through `FUN_0050c9f0`, which adds then clamps, as the port's
`uprising::apply_support_change` does:
- `0x384` (`FUN_0050caa0`): in uprising (`+0x88` bit 2), GNPRTB 7697 (-1).
  Armed by `FUN_00510f20`, the uprising hook.
- `0x385` (`FUN_0050cad0`): with `+0x88` bit 8, side bits 2 and no
  uprising, GNPRTB 7691 (0).
- `0x386` (`FUN_0050cb20`): while blockaded (bit 5), toward the blockading
  side (`FUN_00509890`: its fleet's side bits; `FUN_00509980`: whether
  it is the holder). The amount is GNPRTB 7693 (1), or 7695 (-1) when it
  is not the holder. The period record is system `+0x54`+`0x30`; the port
  reads GNPRTB 7694 (30) for it, and `FUN_005109f0` reads it too.

Missions, uprisings and other events also move the support through the
same path (`uprising.rs`, `support_change`).

## Against the port

- Step 1, the support drift, was invented: it applied the smuggling
  percent to the support every tick. Removed 2026-10-17.
- Steps 0a, 2, 3, 4 and 6 recompute every tick what the original
  recomputes on a change. Their values only change with their inputs, so
  the per-tick run gives the same numbers, except where a value feeds back
  into the next tick (the support).
- Step 7's incidents and the maintenance shortfall (`0x304`, a 30-tick
  counter) have their own sources (`uprising-incident.md`, F-029); the
  counter is the port's.
- SmugglingPercent is ported (step 1). Its readers are not: the
  manufacturing cycle's discarded roll (`side-resource-monitors.md`) and
  the Smuggling message (`0x145`, `message-pictures.md`).

## Open

- Who calls `FUN_00510820` and `FUN_00511740`.
- The period records of `0x384`..`0x386` (`timer-scheduler.md`, "Open").
- `FUN_0050ac80`, `FUN_0050b890`, `FUN_0050c350`, `FUN_0050ac10` and the
  blockade hook's `FUN_0050c0b0`..`FUN_0050c1e0`.
