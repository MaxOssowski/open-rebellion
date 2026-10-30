# Agent Build orders (menu 0x110/0x111/0x112, module 0x17)

Sources: the typed decompile corpus (`FUN_*.c`), plus `objdump -d -M intel`
of GOG `REBEXE.EXE` where the decompile drops `this` or arguments (marked
"asm"). TEXTSTRA.DLL was read directly for the advice text. `hyp:` marks
inferences. Port: `crates/rebellion-core/src/agent_build.rs`.

## 0. Summary

The Build items do **not** queue an automatic build. Each one is a player
directed, one-shot request in three steps:

1. The menu item writes the build kind into module 0x17 and puts the galaxy view
   into **target-cursor mode** (the same mode-2 cursor Move orders use).
2. The player clicks a **destination**: a system, a fleet or a capital ship. The
   module stores it and starts running.
3. The agent finds the **nearest idle producer** of the right type: shipyard,
   training facility or construction yard. It dispatches a **Destination
   (0x214)** order that points that producer's output at the clicked target.
   Then it creates a **Build order** (0x211 ships, 0x212 troops, 0x210
   facilities) on the producer and opens the standard **Build Selection
   window** (bitmap 10800, 210x261) on it. **The player picks the class and
   the quantity** (1..255) and confirms. The dialog dispatches the order.

TEXTSTRA advice 26, "Giving Your Agent Build Orders", confirms this. It is at
file offset 0x15a16 in TEXTSTRA.DLL for the Alliance and 0x19e02 for the Empire:

> "...you can direct me to locate an idle facility and build that unit to a
> destination of your choosing. ... select 'Build Ships', 'Build Troops' or
> 'Build Facilities' from my pop-up menu, and use the target cursor to select a
> destination to which the unit should be built. The Build Window will then
> come up, from which you should select the unit and quantity of that unit
> which you wish me to build. Once you have made your selections, click on the
> check-mark at the bottom of the build window so that I may act upon your
> request."

## 1. Menu and accelerator path

### FUN_00486fb0 (view command handler)
- Lines 112-120: `if (param_2 < 0x113) { if (0x10f < param_2) { FUN_00439d10(param_2); FUN_0041d5e0(0); return; } }`.
  So 0x110, 0x111 and 0x112 all take this path.
- Lines 140-148: cases 0x115..0x118 call `FUN_00439d60(param_2)`. FUN_00439d60
  handles only 0x115 and 0x116. For 0x117 and 0x118 `iVar1` stays 0, so
  **0x117 and 0x118 do nothing**. Their labels are "Manage Maintenance" and
  "Not An Operation" (agent_menu.rs). FUN_00487900 never adds them to the menu.
- For comparison, cases 0x210, 0x211 and 0x212 in the same handler (the object
  pop-up Build commands) call `FUN_0041d640(obj, 0x2a / 0x28 / 0x29)`. The
  agent reuses exactly this Build Selection entry point (section 3, phase 3).

### FUN_00439d10(agent, cmd) (asm 0x439d10..0x439d55)
- If `agent+0x10` (active) is set, it loads `ecx = [agent+0x184]` (module 0x17)
  and calls `FUN_0049e360(kind)`:
  - 0x110 Build Ships → kind **2**
  - 0x111 Build Troops → kind **3**
  - 0x112 Build Facilities → kind **1**

### FUN_0049e360(module, kind) (asm)
- `module+0x34 = kind`
- `FUN_004ece80(module+0x3c)` and `FUN_004ece80(module+0x40)`: each writes 2,
  the null DatId (family byte 0).
- It does **not** change the state (`+0x1c`). The module stays in state 2
  (idle) until a target is clicked.

### FUN_0041d5e0(0) → FUN_00429320(view = DAT_006b14cc, 0)
- `view+0xc0 = 2` (target-cursor mode; `param_1[0x30]` in FUN_00422ce0)
- `view+0xc4 = 0` (no pending order object; `param_1[0x31]`)
- `FUN_00601080(view+0x18)` (hyp: SetCapture)
- `+0x46c = +0x468`

### The click (FUN_00422ce0, WM_LBUTTONUP 0x202, mode 2, lines ~811-880)
- It finds the child window under the cursor and calls its vtable `+0x70(x,y)`
  to get the object DatId under the cursor (hyp: a system on the map, or a fleet
  or ship in a window).
- With `param_1[0x31] == 0` it calls `FUN_0041ce40(&id)`, then
  `FUN_00436070`, `FUN_00487810` (needs the agent at `+0xc0`), `FUN_00439cf0`
  (needs `agent+0x184`), and finally **`FUN_0049e380(module, &id)`**.
- Afterwards mode goes back to 1 and the capture is released.
- Message 0x15e (lines 728-737) cancels mode 2 without telling the agent. The
  module keeps its kind but stays in state 2, which is harmless.

### Accelerators (FUN_00422ce0 lines 1416-1433)
- `0xbc1`, `0xbc2` and `0xbc3` (Alt+B, Alt+T, Alt+F) call
  `FUN_0041cdf0(0x110 / 0x111 / 0x112, ...)`.
- Each is gated on `bVar20` (no tactical battle, `DAT_006be4e4 == 0`), on
  `param_1[0x30] == 1` (not already targeting), and on the menu item existing
  with `item+0x1c` (enabled) non-zero. The same pattern serves 0x113, 0x115,
  0x116, 0x11b and 0x11e.

## 2. Enabled state

- **FUN_00487c20(view)**: `view+0xc0` (agent) ? `agent+0x10` : 0. This is the
  agent's active flag.
- **FUN_00487c30(view)** → **FUN_00439cd0(agent)** (asm 0x439cd0):
  `agent+0x184 != 0 && *(agent+0x184 + 0x1c) == 2`. Module 0x17 must be in
  state **2 = idle**, meaning no build request is pending.
- **FUN_00487900**: `build = active & idle_0x17`. It calls
  `FUN_00520690(0x110/0x111/0x112, build)`, and
  `FUN_00520690(0x115/0x116/0x11b/0x11e, active)`, `(0x113, 1)` and
  `(0x114, 1)`.
  - From the click until the request ends, the module is in state 0 or 1, so
    **all three Build items are greyed**. They come back when the module
    returns to state 2: on success, on failure, or on a cancelled target.
  - `FUN_00439e30(0x117)` always returns 0, because only 0x115 and 0x116 map
    to a module. The "check 0x117" call is dead code.

## 3. Module 0x17

### Constructor FUN_0049e1c0, object size 0x58 (factory FUN_0049e010)
- `FUN_004c7d20(0x17, side)` (base):
  - `+0x18 = 0x17` via FUN_004ec940 (node key)
  - `+0x20 = 0`, `+0x24 = 0`, `+0x28 = 0`, `+0x30 = 0`
  - `+0x2c = side`
  - base vtable 0x65c670
- `+0x3c` DatId = null (2): the **target**, i.e. the player's destination.
- `+0x40` DatId = null: the **work id**. It holds the found sector, then the
  system, then the producer (manager, families 0xa0..0xaf).
- `+0x44`: a list or iterator object (FUN_004355d0), the query result holder.
  It has its own vtable; `+8` saves it and `+0xc` loads it.
- vtable 0x0065bc98
- `+0x54 = 0`: the agent back-pointer, filled by FUN_00439950 (init) and
  FUN_00439550 (load).
- `+0x38 = 0`: the phase.
- `+0x34`: the kind (0 none, 1 facilities, 2 ships, 3 troops), written by
  FUN_0049e360.
- `+0x28`: the context. VT[3] stores `agent+0x3c` there (FUN_0049df20 →
  `(*vt+0xc)(agent+0x3c)`, from FUN_00439950 line 21). It is the agent's own
  galaxy record model, refreshed by FUN_0049cc80 → FUN_004c6770 → FUN_004c6830.

### vtable 0x0065bc98 (7 slots; 0x65bcb4 starts the next class)

| Slot | Address | Role |
|---|---|---|
| 0 | 0x0049e1a0 | destructor |
| 1 | 0x0049e2c0 | save |
| 2 | 0x0049e310 | load |
| 3 | 0x0049eca0 | set_context: `+0x28 = arg` |
| 4 | 0x004cc990 | ready: state 0 → 1, clears `+0x20`; leaves 1 and 2 alone |
| 5 | 0x0049e600 | execute |
| 6 | 0x0049e660 | complete / refuse |

Base vtable 0x65c670, slots 4..6, are `0x618b80`. That is `_purecall`
(`push 0x19; call _amsg_exit`), not a no-op.

### States (`+0x1c`)
- **2, idle.** The initial state (FUN_00439950 sets every module to 2). VT[4]
  never readies it, and the Build items are enabled.
- **0, request accepted.** Set by FUN_0049e380 when a valid target is stored.
- **1, running.** Set by VT[4] in agent tick state 6.
- Every terminal path writes `state = 2`, `+0x20 = 1`, `+0x34 = 0`,
  `+0x38 = 0`, and clears `+0x3c` and `+0x40` in some paths.

### FUN_0049e380(module, &target), the target click (asm 0x49e380..0x49e5f8)
Let `fam = target >> 24`. A target is accepted when it is a system
(0x90..0x97), a fleet (0x08..0x0f) or a capital ship (0x14..0x1b). Family
ranges come from status_rows.rs and object_menu.rs.

- **Accepted, kind 3 (troops), target is a system**, and all of these hold:
  - `FUN_004f3220(side, target)` returns the system;
  - the system holder `(sys+0x24 >> 6) & 3` is not `side`;
  - `FUN_004feea0(sys, side, 1) == 0`. hyp: the side has no fleet (family
    8..0x10) there.

  Then it is **refused**:
  - it creates `FUN_004f5cd0(0x214)` and sets `order+0x20 = side`;
  - it calls `order->vt+0x2c(target)`;
  - it calls `agent->vt+0xc(order, {1, 0x24})`, the advisor refusal reaction;
  - it deletes the order and resets to idle.
- **Any other accepted target**: `FUN_004f26d0(module+0x3c, target)` (asm
  49e558..49e565, ecx = `+0x3c`) stores it, then `state = 0`.
- **fam == 0** (clicked nothing): resets to idle quietly.
- **Any other family**: `agent->vt+0xc(NULL, {1, 0x27})` (asm 49e596..49e5b6;
  ebx = 0, the order is NULL), then idle.
  - hyp: the side handlers FUN_004c3bb0 and FUN_004c19f0, path 0x27/0x25 at
    line 104, call `(*param_2)->vt+0x30` on that NULL order. The original may
    fault here, so a port should only play the reaction.

### Agent tick FUN_00439a10 and module 0x17
- **State 6** runs VT[4] on every module. Module 0x17 goes 0 → 1.
- **State 4**, with no current module: `FUN_0049df70` picks one and the tick
  returns. Execution starts on the next tick.
- **State 4**, with a current module: it runs only if
  `agent+0x188 < agent+0x18c`, **or** `module+0x18 == 0x17` (lines 37-38).
  So 0x17 ignores the 5-order cap. Then:
  - `module+0x30 = 0`;
  - `order = VT[5]()`;
  - when an order comes back, `ok = order->vt+0x1c(status)`;
    - `ok == 0`: VT[6](status, order), a refusal;
    - otherwise `FUN_004878f0(order)` dispatches it, and `agent+0x188++`
      only when the module is **not** 0x17 (lines 50-53).
  - When `+0x20` is 0 it returns and the same module runs again on the next
    tick (one phase per tick). Otherwise it sets `state = 0` unless the state
    is 2, clears current, and moves to state 5.

### VT[5] FUN_0049e600 → FUN_0049e6b0 (phases at `+0x38`)
- If state != 1: reset to idle (`+0x34 = 0`, `+0x38 = 0`, `+0x20 = 1`,
  `+0x1c = 2`) and return 0.
- If kind `+0x34 == 0`: same reset, return 0.
- **Phase 0 or other**: `+0x38 = 1`, return 0.
- **Phase 1** (asm 49e756):
  - With `+0x3c` null: `+0x20 = 1` and return. The module becomes state 0
    again and re-runs.
  - Otherwise `FUN_0049e7a0`. Success: phase 2, return 0. Failure: idle, with
    the reaction already sent inside.
- **Phase 2**: `FUN_0049e960` returns a Destination order.
  - Non-null: `+0x38 = 3`, `+0x20 = 1`, `+0x30++`, and the order is
    **returned**. The tick validates and dispatches it. The module goes
    state 0, then 1 on the next state 6.
  - Null: idle, with no message.
- **Phase 3** (asm 49e6d6..49e728):
  - `kind = FUN_0049eb50()`: 1 → **0x210**, 2 → **0x211**, 3 → **0x212**.
  - `o = FUN_0049ea40(kind)`. The work id `+0x40` must be family 0xa0..0xaf.
    It calls `FUN_004f5cd0(kind)`, sets `o[8] (+0x20) = side`, builds an
    object list of `+0x40` (FUN_004f5b10 / FUN_004f57b0) and sets it with
    `o->vt+0x24`. **No class and no quantity are set.**
  - `FUN_0049ec40(o)`: kind 1 → `FUN_0041d640(o, 0x2a)`, 2 →
    `FUN_0041d640(o, 0x28)`, 3 → `FUN_0041d640(o, 0x29)`. FUN_0041d640 →
    FUN_00437df0 opens **Build Selection**: 0x154 bytes, 210x261, background
    0x2a30, `DAT_006b28ac`. It is filtered to the producer's buildable classes
    (FUN_00537ff0). Quantity starts at 1 and is clamped to 1..255. Confirm
    copies the class and quantity into the order and calls
    `FUN_0041ce20(order, 0)`, which leads to FUN_00487740 (see
    manufacturing-build-selection.md).
  - `DAT_006b28b0 |= 0x10000000`.
  - `FUN_0043a290(agent = [module+0x54], o)`: **`agent+0x180 = o`**.
  - Idle (`+0x38 = 0`, `+0x34 = 0`, `+0x20 = 1`, `+0x1c = 2`), return 0.
    The tick dispatches nothing; the dialog does.
  - If FUN_0049ea40 returns NULL: idle, no message.

### Phase 1 producer search, FUN_0049e7a0 (asm 0x49e7a0..0x49e95a)

1. `m1 = FUN_0049eb80()`: kind 1 → 0x4000, 2 → 0x1000, 3 → 0x2000.
   `FUN_0049d770(ctx = +0x28, &+0x3c, m1 | 0x10, 0, 0, 0, order 2)` lists the
   **sector records** by distance from the target's sector. It climbs the
   target's containers to family 0x80..0x8f and uses the positions at record
   `+0x2e`, keyed by FUN_0041b7b0, the squared distance. A record passes when
   `(rec+0x24 & m1|0x10) == m1|0x10`; the other masks are 0.
   `+0x44 = list`, `+0x40 = FUN_004357b0(list)` (hyp: the nearest, as in
   ai-galaxy-records.md "By distance"), then `FUN_005f58b0(+0x44)` (clear).
2. `m2 = FUN_0049ebc0()`: kind 1 → 0x200, 2 → 0x80, 3 → 0x100.
   `FUN_0049da70(ctx, &+0x40 (sector), &+0x3c (target), m2 | 4, 0, 0, 0, 2)`,
   which calls FUN_004c6460, lists the **system records** of that sector by
   distance to the target. `+0x40 = pick`, then clear.
3. The pick must be a system (0x90..0x97). Then
   `sys = FUN_004f3220(side, +0x40)` and
   `mgr = FUN_0049ec00(sys)` = `FUN_00509670(sys, area)` with area 1 for
   facilities, **0** for ships and 2 for troops (asm 49ec00..49ec38).
   `+0x40 = mgr's DatId` (FUN_004025b0) and the result is 1.
4. On failure: `agent->vt+0xc(NULL, {0xa0, 3})`, return 0.

**Record bits** (player agent records):
- System record `+0x24`, set in FUN_004ebe40:
  - 4: holder == side; 8: the opponent holds it; 0x10: neither side.
  - 0x80 needs all of: at least one shipyard (`+0x54`) with an active bit
    (`+0x50 & 1`), no uprising or blockade (`+0x24 & 3 == 0`), and
    `FUN_00509670(sys, 0)->RemainingGameObjCount == 0`. That last condition
    makes the producer idle (asm 4ec1fd: push 0).
  - 0x100: the same test for training facilities (`+0x58`, area 2,
    asm 4ec2e3).
  - 0x200: the same test for construction yards (`+0x5c`, area 1,
    asm 4ec3cb).
  - 0x400, 0x800 and 0x1000 mark producers that are Reserved or hold more than
    one yard.
- Sector record `+0x24`, set in FUN_004c6830 from each own system (system bit 4):
  - `0x10`
  - `0x1000` ← system 0x80, `0x2000` ← 0x100, `0x4000` ← 0x200

So "nearest available":
- Sector: the nearest one with an own system that has an idle producer of the
  kind.
- System: inside that sector, the nearest own system to the target with such an
  idle producer.
- Producer: that system's production manager of the kind.

hyp: the "nearest" reading relies on order 2 plus FUN_004357b0 taking the last
entry, as documented for the AI queries. The player-side builders
FUN_0049d770 and FUN_004c6460 have the same structure.

### Phase 2 Destination, FUN_0049e960
- Needs `+0x3c` and `+0x40` both non-null.
- `o = FUN_004f5cd0(0x214)` (Destination; move-order.md "Order 0x214").
  The object list is `[+0x40 manager]` (FUN_004f5b10 / FUN_004f57b0, through
  `vt+0x24`), `o+0x20 = side`, `o->vt+0x2c(+0x3c)` (destination = the
  player's target).
- This repoints the producer's delivery to the clicked target.
- If the validator (`vt+0x1c`) refuses it, VT[6] FUN_0049e660 runs:
  - `+0x34 = 0`, `+0x38 = 0`, `+0x20 = 1`, `+0x1c = 2`, `+0x30 = 0`;
  - `agent->vt+0xc(order, status)`, the refusal reaction;
  - the order is deleted.

## 4. Advisor messages and DAT_006b28b0

`agent->vt+0xc` is `_purecall` in the base (0x658e10). The real ones are
0x004c2940 (vtable 0x65c4c8, FUN_004c27f0, side 1) and 0x004c0870
(0x65c4a0, FUN_004c0710, side 2); see mission-dialog.md "Refusal". The
status pair `{word0, word1}` selects a reaction slot in the `agent+0x168`
due-time table. Slot `k` = offset/4, which is the side's `.SPT` message `k`
(mission-dialog.md). **No text strings are involved: the droid reacts.**

| Status | Source | Side 1 (C3PO) slot | Side 2 (IMP-22) slot |
|---|---|---|---|
| `{0xa0, 3}` no idle producer found | FUN_0049e7a0 | `+0x60` (FUN_004c3860) | `+0x64` (FUN_004c16a0) |
| `{1, 0x27}` bad target family | FUN_0049e380 | FUN_004c3bb0: `+0x1c` if the order target is a fleet, else `+0x5c` (NULL order, see hyp above) | FUN_004c19f0: `+0x1c` / `+0x60` |
| `{1, 0x24}` troops to an unheld system with no fleet | FUN_0049e380 | FUN_004c3bb0 line 202: `+8` (target is a system) | FUN_004c19f0: `+8` |
| any Destination validator status | VT[6] | the generic table in mission-dialog.md | same |

**Bit 0x10000000**, set in FUN_0049e6b0 phase 3, means "the agent's Build
dialog is pending" (the order is at `agent+0x180`). It does *not* mean a move
order was dispatched.
- **FUN_0043a2a0(agent, order)** is called from FUN_00487740 after a
  successful UI dispatch. If the bit is set and `order == agent+0x180`, it sets
  `DAT_006b28b0 |= 1`. It then always clears 0x10000000.
- **Bit 1** is consumed by FUN_004c2dd0 (hyp: side 1) as slot `+0x8c`
  (message 0x23), 10 ticks later, and by FUN_004c0d00 (hyp: side 2) as slot
  `+0x70` (message 0x1c). This is the "request acknowledged" reaction.
- hyp: if the player cancels the dialog, the bit stays set until the next
  dispatched order clears it, and no acknowledgement plays.

TEXTSTRA: the only text is advice 26, "Giving Your Agent Build Orders"
(RCDATA `0x6001 + 3*26` / `0x6002 + 3*26` = 0x604f / 0x6050; Empire
0x684f / 0x6850, per agent-advice-list.md). There is no "no shipyard
available" string.

## 5. Save and load

- **VT[1] FUN_0049e2c0** (asm): base FUN_004c7d70, then `+0x38` (phase),
  `+0x34` (kind), DatId `+0x3c` (FUN_004ece90), DatId `+0x40`, and the list
  `+0x44` (`vt+8`).
- **VT[2] FUN_0049e310**: the same order through FUN_004c7dc0, FUN_005f4db0,
  FUN_004ecea0 and list `vt+0xc`.
- **Base FUN_004c7d70**: Class_0065ca30_vt4 (FUN_005f5610 node key, then
  `+0x1c` state through FUN_005f4e10), then `+0x2c` side, `+0x20` completed,
  `+0x30` priority and `+0x24`.
- **Agent load FUN_00439550** (lines 63-65) re-finds module 0x17:
  `agent+0x184 = it` and `it+0x54 = agent`.
- `agent+0x180`, the pending Build order, is **not** in FUN_004397a0's list.
  hyp: a save made while the dialog is open loses the acknowledgement link.

## 6. Contradictions with manage-automation.md (and code comments)

1. **The 0x17 role.** §2 and §12 label 0x17 "Build Ships" and claim it is an
   autonomous enabled automation. It is in fact the one-shot module behind all
   three Build items (kind at `+0x34`: 1 facilities, 2 ships, 3 troops). It is
   never a toggle, and state 2 means idle.
2. **Order numbers.** In §7 and §8, and in agent_automation.rs (lines 153 and
   159), **0x214 is Destination**, not "Build-Facility". 0x210 builds
   facilities, 0x211 ships and 0x212 troops. Production phase 3 is a
   Destination order and phase 4 the Build order.
3. **Tick branches swapped.** §6 says "If compatible: VT[6], else dispatch".
   FUN_00439a10 lines 44-49 do the reverse: a validator result of 0 means
   VT[6] (refusal), non-zero means dispatch. Also, picking a module returns at
   once, and execution starts on the next tick.
4. **The base vtable.** In §2 the base slots 4..6 are `_purecall` (0x618b80),
   not "nop". The vtable table also lacks the 0x17 column given above.
5. **Agent fields.** In §1, `+0x3c` is the agent's galaxy record model (the
   module context), not a notification queue. `agent+0x180` is missing (the
   pending Build order). The module's `+0x2c` is the side number.
6. **Bit 0x10000000.** §9's "Ships module dispatched a move order" is wrong;
   see section 4 above. Bit 1 (agent build acknowledged) is missing from the
   table.
7. **Menu items 0x117 and 0x118.** The Open hyp (0x117 = Build Ships) is wrong.
   FUN_00439d60 and FUN_00439e30 have no cases for them, so they are no-ops,
   and FUN_00487900 does not add them to the menu.
8. **The 0x110..0x112 handler.** §4 implies the Build items reach module state
   through FUN_00439d60. They actually go through FUN_00439d10, then
   FUN_0049e360, then the FUN_00429320 target cursor.
9. **The manual's Alt+B line.** The quote in the note ("build a ship using the
   nearest available shipyard") is right about the yard search but leaves out
   the destination cursor and the Build Selection window, both in advice 26.

## 7. A0: when the agent runs (2026-10-30)

The agent tick `FUN_00439a10` is a work step: it returns 0 while a module
still has phases to run and 1 when a cycle ends. Its caller
(`Class_0065ad70_vt24`) is not traced. In the Windows VM (Empire, day 208,
Very Slow), Build Troops on Coruscant opened Build Selection in under a
second and the day did not change. At Medium it opened within the same day
too. So the request does not wait for a day: the port runs its phases at
once.

The same session also showed two other things:
- Build Ships on Ghorman opened nothing, and the Build items were enabled
  again afterwards (no idle shipyard, `{0xa0, 3}`).
- Build Selection stays open while the clock runs.

Captures are in `.artifacts/a0-vm/build-orders/` (not committed); their
hashes are in the evidence folder.
