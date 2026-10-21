# The economy's cadence: what changes a system, and when

Read 2026-10-17 for F-037. Every tick the port runs `EconomySystem`,
which rebuilds each system (until 2026-10-17 it also drifted its
support). The original has no such loop:
- `FUN_00508250` (system slot `+0xc0`) runs once, in the setup stage
  `0x17` (`timer-scheduler.md`). Ported 2026-10-21 as
  `EconomySystem::setup`, run at game creation (the app's new game, the
  playtest runner and the seed-42 fixture). Before it, the first day's
  run found every field changed from its default and reported it.
- After that a system changes only through its field hooks (when an input
  changes) and its timers.

## The derived fields and who computes them

Each of these setters stores a field and notifies its views and its master
slot (`ghidra/generated/setters.tsv`).

| Field | Setter | Computed by | Value | Port |
|---|---|---|---|---|
| `+0x6c` SmugglingPercent (-100..100) | `FUN_00509ec0` | `FUN_0050b230` → `FUN_00559c40` | see below | step 1, ported 2026-10-17 (it had been misread as a support drift) |
| `+0x70` ProductionModifier | `FUN_00509f40` | `FUN_0050b2c0` → `FUN_00559c10` | GNPRTB 7763 (`DAT_006bb44c`, 100) × 100 / the holder's support (1 for 0), integer division; 100 with no holder | `economy::production_modifier`, the field `collection_rate`; feeds the mines' and refineries' cycles (2026-10-17) |
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
- `FUN_0050b230` has no populated test. Since 2026-10-21 a system held
  only by regiments gets its smuggling percent, and its holder is
  resolved (`evidence/2026-10-21-unpopulated-holder/`,
  `evidence/2026-10-21-unpopulated-smuggling/`). port: the economy's
  other fields still skip unpopulated systems.
- Bit 11 of `+0x88` is DeathStarNearby (see "Bit 11" below), ported
  2026-10-17 in place of the port's support guess.

## The hooks

Each System setter notifies a master slot (`setters.tsv`), and the System
vtable `0x0065e640` holds that slot's hook (`class-slots.tsv`). Read
2026-10-17; the earlier reading had the first one as the regiments' hook.

| Slot | Field that changed | Hook | It recomputes |
|---|---|---|---|
| `+0x1d0` | `+0x58` support (`System_SetLoyalty`) | `FUN_005109f0` | holder (`FUN_0050b610`), TroopRegRequired (`FUN_0050b5a0`), SmugglingPercent (`FUN_0050b230`), ProductionModifier (`FUN_0050b2c0`); while blockaded, the `0x386` minimum: GNPRTB 7694 when the blockader is the more popular side, else 7696 |
| `+0x1d4` | `+0x5c` energy | `FUN_00510ae0` | `FUN_0050ad60`, `FUN_0050ad80` |
| `+0x1dc` | `+0x64` raw material | `FUN_00510b20` | `FUN_0050ad60`, `FUN_0050b0b0` |
| `+0x1e8` | `+0x70` ProductionModifier | `FUN_00510b60` | `FUN_0050c2b0` |
| `+0x200` | `+0x7c` TroopRegSurplus | `FUN_00510cf0` | uprising start (`FUN_0050b800`), `+0x88` bit 9 (`FUN_0050bb00`) |
| `+0x204` | `+0x80` TroopRegRequired | `FUN_00510d40` | TroopRegSurplus (`FUN_0050b500`) |
| `+0x208` | `+0x84` control word | `FUN_00510d70` | when the holder changes: bit `0x2000` (held by support alone), `FUN_004f6f40(holder)`, event `0x31b` |
| `+0x20c` | `+0x88` bit 0 populated | `FUN_00510e30` | when set, the support (100 to the holder, or the middle for no side); holder, TroopRegRequired, `FUN_0050ac10(1)`, `FUN_0050b890` |
| `+0x214` | bit 2 uprising | `FUN_00510f20` | `FUN_0050bbb0`, `FUN_0050c180`..`FUN_0050c1e0`, TroopRegRequired, `FUN_0050ba90`, `FUN_0050b4d0`, `FUN_0050bb00`; event `0x303`; arms `0x384`, `0x38d` |
| `+0x220` | bit 5 blockade | `FUN_00511300` | see below; arms `0x386` |
| `+0x238` | bit 11 DeathStarNearby | `FUN_00511740` | TroopRegRequired, SmugglingPercent |

A unit joining or leaving the system runs its slot `+0xd0`,
`FUN_00508660`, by family: regiments (`0x10..0x13`) recompute the holder,
`FUN_0050ba90` and TroopRegSurplus (`FUN_0050b500`); fleets (`0x08..0x0f`)
and squadrons (`0x1c..0x1f`) the postures and side presence; facilities
their own fields. None of these recompute SmugglingPercent or
ProductionModifier.

### Port of the cadence (2026-10-17, step 3b)

- `SystemEconomy::hook_inputs` keeps the support, bit 11 and side seen on
  the last run (save v63). The smuggling percent is recomputed only when
  one of them changes, and the collection rate (ProductionModifier) only
  when the support changes. port: a change made after the economy has
  run is seen on the next day's run.
- The side hook clears the uprising bit: `ActiveUprising::side` records
  the holder, and the revolt ends when the holder changes
  (`uprising.rs`). The port had kept the revolt going under the new
  holder, whose support the revolt timer then wore down.
- TroopRegRequired and the holder have every input among their hooks'
  triggers, so the daily recompute gives the original's values.
- The side hook's informant timer `0x38e` (`FUN_0050c820`) is ported in
  `informant.rs` (2026-10-17, `informant-incident.md`).

### ProductionModifier reaches the facilities (2026-10-17)

The field's hook (slot `+0x1e8`, `FUN_00510b60`) runs `FUN_0050c2b0`. It
walks the system's processing facilities (`FUN_0055b260`, families
`0x2c..0x30`: mines and refineries) and gives each the system's value
(`FUN_0050c560` -> `FUN_0055a7a0`, facility `+0x6c`). A facility joining
the system gets it too (`FUN_00508660`). The facility's own hook runs
`FUN_0055abf0`, which writes the next cycle's period
(`FUN_0055e2b0`: `(rate + extra) × modifier / 100`) into the timer record
without cancelling the running timer, so the change applies from the next
cycle. Shipyards (`0x28..0x2a`) are not in the walk.

So support drives the mines and refineries: at 100 support a cycle takes
its base length, at 50 twice that, at 10 ten times. A system with no
holder runs at 100. The port had set 200 for a system with no holder and
kept every facility at 100.

Port: `economy::production_modifier`, `production_modifier_at`, and
`ProductionSystem::advance`'s `modifier`; a cycle takes its system's value
when it starts. port: a system the economy has not yet run (none before the
first day) counts 100.

So the original recomputes:
- SmugglingPercent only when the support or bit 11 changes (and in
  `FUN_00510820`), not when ships, squadrons or regiments come or go;
- TroopRegRequired when the support, the populated bit, the uprising bit or
  bit 11 changes;
- the holder when the support, the regiments or the populated bit changes;
- everything above, and clears the uprising, when the side changes.

## Bit 11: DeathStarNearby

`System_SetDeathStarNearby` (`FUN_0050a890`) writes `+0x88` bit `0x800`.
Its only caller, `FUN_005178b0`, walks the sectors (`0x80..0x90`). For
each one it looks for an active (`+0x50` bit 0) Death Star (`0x18..0x1c`)
whose system lies in that sector (`FUN_00518690`: the system's parent
`+0x1c`), then sets the bit on every system of the sector to the answer.
`FUN_00514510` and `FUN_00514a60` run it when objects move.
The Death Star's side does not matter.

The bit doubles the Empire's regiments in the smuggling percent
(`FUN_00559b60`) and in the uprising score (`FUN_00559ce0`), halves the
Empire's garrison requirement (`FUN_00559fe0`), and halves support shifts
that favour the Alliance or hurt the Empire (`FUN_00559be0`). The port had
read it as "the holder's support is above GNPRTB 7732 (40)".

Port (2026-10-17): `economy::death_star_nearby` computes it from the
fleets at the sector's systems when read; a fleet in hyperspace stands at
no system.

## The other hooks
- `FUN_00510820` is the System's side hook (slot `+0xf0`, overriding
  `FUN_004fae40`). `FUN_004f6f40` runs it when it changes the side bits
  `+0x24` to the new holder, which `FUN_00510d70` does when the control
  word's holder changes. It runs `FUN_0050ac80`, `FUN_0050b890`,
  `FUN_0050c350`, `FUN_0050a4a0(0)` (clears the uprising bit),
  `FUN_00509ec0(0)`, `FUN_0050b230`, `FUN_0050b5a0`, `FUN_0050c820`
  (arms the informant timer `0x38e`) and `FUN_0050ac10(1)`.
- `FUN_00511300`, the blockade changing: `FUN_0050c0b0`, `FUN_0050bbb0`,
  `FUN_0050c180`, `FUN_0050c1b0`, `FUN_0050c1e0` and `FUN_0050b310` (the
  withdraw percent), then `FUN_0050ce20` for both sides. It arms timer
  `0x386`.

## The timers that change support

All go through `FUN_0050c9f0`, which adds then clamps, as the port's
`uprising::apply_support_change` does:
- `0x384` (`FUN_0050caa0`): in uprising (`+0x88` bit 2), GNPRTB 7697 (-1)
  to the holder (side bits). Armed by `FUN_00510f20`, the uprising hook,
  on record `+0x54`+`0x20`, whose minimum `FUN_0050e200` sets to GNPRTB
  7698 (30).
- `0x385` (`FUN_0050cad0`): with `+0x88` bit 8, side bits 2 and no
  uprising, GNPRTB 7691 (0).
- `0x386` (`FUN_0050cb20`): while blockaded (bit 5), to the blockading
  side (`FUN_00509890`: the side bits of the first fleet at the system).
  The amount is GNPRTB 7693 (1) when that side has more support, else
  7695 (-1): `FUN_00509980` compares it with `FUN_00509930`, which names
  the side with more support (`FUN_00507270`), or 3 on a tie. Armed by
  `FUN_00511300`, the blockade hook, on record `+0x54`+`0x30`, whose
  minimum `FUN_0050e200` sets to GNPRTB 7694 (30); `FUN_005109f0` resets
  it, when the support changes, to 7694 when the blockader is the more
  popular side, else 7696 (30).
  The economy's maintenance counter also reads GNPRTB 7694 (F-029).

Arming (`FUN_0053fa60`) stores the flag's new value in the record's armed
bit; when that changes the record, the old timer is cancelled (its counter
no longer matches) and, if armed, a new one is scheduled. So both timers
start when the uprising or blockade starts and stop when it ends. A fired
timer reschedules with the same delay.

### Port of the timers (2026-10-17)

- `uprising.rs`: `ActiveUprising::next_support_tick`; `UprisingEvent::
  SupportEroded` carries the holder's change after `FUN_00559be0`.
- `blockade.rs`: `BlockadeState`'s timers, set when a blockade starts and
  dropped when it ends; `BlockadeEvent::SupportShifted`.
- Save v62. A change of 0 posts no event.
- port: the spread draw over `0..=0` takes no roll; `FUN_005109f0`'s reset
  to 7696 is not modelled (both 30).
- hyp: the delay records' spreads are 0 (`timer-scheduler.md`).
- `0x385` is not ported: its change, GNPRTB 7691, is 0.

Missions, uprisings and other events also move the support through the
same path (`uprising.rs`, `support_change`).

## Against the port

- Step 1, the support drift, was invented: it applied the smuggling
  percent to the support every tick. Removed 2026-10-17.
- Steps 0a, 2, 3, 4 and 6 recompute every tick what the original
  recomputes on a change. Their values only change with their inputs, so
  the per-tick run gives the same numbers, except where a value feeds back
  into the next tick (the support).
- Step 7's incidents have their own sources (`uprising-incident.md`,
  F-029). The maintenance shortfall (`0x304`) is the side's autoscrap
  timer `0x382` (`side-resource-monitors.md`, "Autoscrap"); the port's
  30-tick counter, which had no source, is gone (2026-10-17).
- SmugglingPercent is ported (step 1). Its readers:
  - the manufacturing cycle's discarded roll (`side-resource-monitors.md`)
    is not ported;
  - the Smuggling message (`0x145`, `FUN_00499130`) is ported 2026-10-20
    as `story_messages::smuggling_message`. It speaks when the percent
    begins or ends, by whose system it is (`story-message-classes.md`).

## Open

- `FUN_0050ac80`, `FUN_0050b890`, `FUN_0050c350` and `FUN_0050ac10`, run
  by the side hook.
- `FUN_0050ac80`, `FUN_0050b890`, `FUN_0050c350`, `FUN_0050ac10` and the
  blockade hook's `FUN_0050c0b0`..`FUN_0050c1e0`.
