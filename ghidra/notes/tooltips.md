---
title: "Tooltips: the shared tip, its timing and look, and every tool the game registers"
description: "FUN_006008e0's tooltips_class32 control, the A0 measurements of its tip, and the inventory of all ~262 tooltips by window"
category: ghidra
created: 2026-10-30
updated: 2026-10-30
tags: [tooltip, interface, command-center, finder, message-index, tactical]
---

# Tooltips

## The shared tip (port: `crates/rebellion-render/src/win_tooltip.rs`)

`FUN_006008e0` creates one `tooltips_class32` window (`DAT_006be5a4`) for
the whole game, sends `TTM_ACTIVATE(1)` (0x401) and
`TTM_SETDELAYTIME(TTDT_AUTOMATIC, 500)` (0x403; its only caller,
`FUN_004138a0`, passes 500), and installs a `WH_GETMESSAGE` hook
(`lpfn_00600e50`) that relays the mouse messages. `TTDT_AUTOMATIC` 500 gives
an initial delay of 500 ms, an autopop of 5000 ms and a reshow of 100 ms.
`FUN_00600e20` unhooks it.

A0 (Windows 10 VM, Empire save, `.artifacts/a0-vm/tooltip/`, hashes in the
evidence README):

| Capture | What it shows |
|---|---|
| `01.png` | Pointer at (175, 25) on the Raw Materials monitor: tip "Raw Materials Monitor", box (175, 46) 124 by 19 |
| `02-nocursor-fast.png`, `03.png` | Pointer at (260, 25): no tip at once; after 2.5 s "Refined Materials Monitor", box (260, 46) 143 by 19 |
| `04-4s.png`, `05-6s5.png`, `06-moved.png` | Shown at 4 s, gone at 6.5 s, and not back after a move on the same control |
| `08-index-hover-monitor.png` | Message Index open (modal): no tip on the monitor |
| `09-index-tab.png`, `10-index-sort.png` | Inside the Index: "Popular Support Messages" on a tab, "Select All" at (440, 169) for the pointer at (440, 148) |
| `11-maintenance.png`, `12-speed.png`, `13-gap-244.png` | Maintenance Monitor and Game Speed Control at the pointer plus (0, 21); no tip at x 244, between the Raw and Refined rectangles |
| `20-sysfinder.png` .. `26-gamecontrols.png` | The cockpit's controls: near the bottom the tip goes above the pointer (pointer y 440, tip 421..439) |
| `25-gid.png`, `27-rail-support.png`, `28-rail-chat.png` | Near the right edge the tip moves left; its border's last column is 638, so it ends at 639 |

The tip: fill (255, 251, 240) (`COLOR_INFOBK` in 256 colours), a one-pixel
(128, 128, 128) border, black text whose ink starts 4 pixels in and whose
capitals start 5 pixels down and are 9 high. Its top left is the pointer's
hot spot plus (0, 21). The width is the text plus about 5 pixels. The tip
stays one pixel inside the 640 by 480 screen: past x 639 it moves left to
end there, and past y 479 it goes above the pointer, ending at the pointer's
row (each `*-base.png` is the same spot right after the move, before the
tip, for the difference).

The port draws the text with the Arial-metric face at an em of 12; egui
draws it about 9% wider than GDI (the accepted text-width deviation), and
the tip fits its text, so the port's tips are as much wider.

## Summary

- **Count.** About 262 tools. The 230 table rows mean about 262 tools once
  loops (6 save, 6 load, 8 task-force, 4 fighter-group, 4 nav-point buttons)
  and the fleet window's two strips are expanded. Faction variants that make
  the same tool count once. The 3 sector-window bars are made per system and
  count once. They sit in 33 windows or panels across 9 groups. 14 functions
  change tooltip text at run time with TTM_UPDATETIPTEXT (0x40c), touching 32
  tools (7 of them are Message Index category buttons that are only reset). There are 3 delete paths.
- **Groups:** shell (main menu, setup, game options, dialogs); command center
  chrome and cockpit; finders; message index; encyclopedia; status and
  summary windows; galaxy windows (sector, fleet, system defenses, missions,
  manufacturing, build selection, move order); mission dialog; tactical
  battle.
- **No layout table.** Nothing reads tooltip ids from a resource or layout
  table. Every id is an immediate value in the window's create routine. Four
  helpers carry ids through, and their callers are listed below:
  - `FUN_00606c90`, a bitmap button: its 6th explicit argument is a string id
    for `FUN_006009f0`.
  - `Class_0066dbc8_vt4c` (`FUN_005ef240`), the tactical "add child": its 3rd
    argument is a tooltip id struct.
  - `FUN_00417020` / `FUN_00401150`: a REBDLOG message box. The button label
    id is also its tooltip.
  - `FUN_00416c50` / `FUN_00401110`: the REBDLOG Yes/No box.
- **Uncertain (marked inline):**
  - the Load-row tooltip in Game Options, where the decompile drops the
    button's creation;
  - the doubled `0x72` tab in Battle Summary;
  - the exact assembly order of the runtime text for sector-window bars;
  - the window names for `FUN_0040dfd0` and some tactical panels, which come
    from their strings and not from notes.

## Mechanism (the porting rules)

- **One control.** One shared `tooltips_class32` (`DAT_006be5a4`), created in
  `FUN_006008e0`, TTDT_AUTOMATIC 500 ms.
- **Adding a tool:**
  - `FUN_00600970(owner=ECX, control, text)` makes an HWND tool
    (`TTF_IDISHWND`, `uFlags=1`, `uId = control+0x18`). It copies the text to
    `control+0x84` and stores the owner HWND at `control+0x90`.
  - `FUN_00600bd0(rect, id, text)` makes a rect tool (`uFlags=0`,
    `hwnd = owner+0x18`, `uId = id`).
- **Two ways to load the text:**
  - **A. `FUN_006009f0` / `FUN_00600ac0` (+ `FUN_00606c90`).** These call
    `LoadStringA(DAT_006be5ac, id)` (`FUN_00601060`). `DAT_006be5ac` is the
    current window's string module. `Class_0066e060_vt34` (`FUN_006068f0`)
    sets it from window `+0x94`, the 7th constructor argument of
    `FUN_00606380`, before it calls the create slot `+0x38`.
  - **B. `FUN_00600a40` / `FUN_00600b30` / `FUN_00600c40`.** These take a
    4-byte struct `{u16 id; u16 module}` by value. `FUN_005f2fc0` →
    `FUN_005f37e0` → callback `DAT_006bde14` = `FUN_0060aa30` →
    `LoadStringA(FUN_005fefd0(module), id)`. The module words are
    `DAT_0065d420` = 1 (TEXTCOMM), `DAT_0065d424` = 2 (TEXTSTRA) and
    `DAT_0065d428` = 3 (TEXTTACT), read from `.rdata`. Tactical code keeps the
    structs in `.bss` globals. Tiny static-init stubs fill them, for example
    `mov ax,[0x65d428]; mov word [0x6bcd2c],0xddba; mov [0x6bcd2e],ax`. Ghidra
    has no functions for these stubs, so they were resolved by scanning the
    EXE bytes for `66 C7 05 <addr> imm16`.
  - String modules by index: 1 TEXTCOMM, 2 TEXTSTRA, 3 TEXTTACT, 12 REBDLOG.
    The last is inferred: the dialog constructor passes 0xc, and REBDLOG holds
    the 0x1300/0x190d.. ids.
- **Updating text:** `FUN_00600cc0` (TTM_UPDATETIPTEXT), or `FUN_00600c40`
  (path B plus update).
- **Deleting tools:**
  - `FUN_00600d30` (TTM_DELTOOL for an HWND tool) runs from the control
    destructor `FUN_005ffb60` and from WM_DESTROY in `FUN_00600310`, so every
    tool dies with its control.
  - `FUN_00600db0(owner, id)` deletes a rect tool. Its only caller,
    `FUN_0045d140` (the shared tail of the sector-window item refresh),
    deletes id `item+0xc` when the item's state goes to 0. No traced code adds
    a rect tool to the sector window, so this is probably a no-op or
    vestigial (guess).
- **Other paths.** No other code sends to `DAT_006be5a4`. The close-button
  helper `FUN_00606e10(art, pressed art, command, tip)` passes its fourth
  argument to `FUN_00606c90` as the tip. Its only caller, Build Selection's
  `FUN_00438620`, passes `0x1903` "Close"; A0 build-selection/tip-close
  shows it (corrected 2026-10-31; this line said the helper registers
  none).

Notation:

- "Cmd" is the control/command id, "pos" the client position (x, y) and
  size, both hex as in the decompile.
- "A/E" means Alliance/Empire variants.
- Ids are given as hex (decimal).

---

## 1. Shell: main menu, game setup, game options, dialogs

### 1a. Main menu (`FUN_00405560`, TEXTCOMM via `DAT_006be5ac = FUN_006037f0(1)` in `FUN_00405050`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| +0xac cmd 0x6a, 3-state `FUN_00603cd0` at (0xf2,0x10f) 0x2c×0x2f | 0x1001 (4097) | Select Galaxy Size | FUN_00405560 L182 | static |
| +0xb4 cmd 0x65 at (0x1b5,0x133) | 0x1002 (4098) | Start Game as Alliance | L187 | static |
| +0xb0 cmd 0x66 at (0x99,0x134) | 0x1003 (4099) | Start Game as Empire | L192 | static |
| +0xc4 cmd 0x67 at (0x43,0x17d) | 0x1004 (4100) | Play Head to Head | L202 | static |
| +0xc8 cmd 0x68 at (0x19b,0xe8) | 0x1005 (4101) | Load a Saved Game | L197 | static |
| +0xcc cmd 0x69 at (0x218,0x189) | 0x1006 (4102) | Exit | L207 | static |
| +0xb8 cmd 0x6e at (0x3d,0x29) | 0x1007 (4103) | Novice | L221 | static |
| +0xbc cmd 0x6f at (0x7c,0x28) | 0x1008 (4104) | Intermediate | L231 | static |
| +0xc0 cmd 0x70 at (0xbb,0x29) | 0x1009 (4105) | Expert | L241 | static |
| +0xa8 cmd 0x71 at (0x131,0x14d) | 0x100a (4106) | Victory Conditions | L214 | static |
| +0xa4 cmd 0x73 at (0x1cb,0xf2) | 0x100b (4107) | Credits | L177 | static |
| rect tool id 0x6b, (0x122,0x125)-(0x13a,0x13a) | 0x100f (4111) | Standard Galaxy | L272 (`FUN_00600ac0`) | static rect tool over the galaxy-size indicator |
| rect tool id 0x6c, (0x146,0x125)-(0x15e,0x13a) | 0x100e (4110) | Large Galaxy | L299 | static rect |
| rect tool id 0x6d, (0x16a,0x125)-(0x182,0x13a) | 0x100d (4109) | Huge Galaxy | L326 | static rect |

### 1b. Game setup screen (`FUN_0040dfd0`, vtable slot at 0x6587c8; TEXTCOMM: the ids exist only there; window name is a guess: multiplayer / new-game setup)

All of these are `FUN_00606c90(x, y, bmp, bmp_pressed, cmd, tooltip, label, ...)`.

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| cmd 100 at (0x8d,0x1ba) | 0x1100 (4352) | Previous | FUN_0040dfd0 L304 | static |
| cmd 0x65 at (0x122,0x1ba) | 0x1101 (4353) | Next | L303 | static |
| cmd 0x66 at (0x122,0x1ba) | 0x1103 (4355) | Done | L305 | same spot as Next; shown alternately |
| cmd 0x67 at (0x1b5,0x1ba) | 0x1102 (4354) | Cancel | L302 | static |
| cmd 0x6f at (0x1f6,199) | 0x1104 (4356) | Load a Saved Game | L306 | static |
| cmd 0x6b at (0x185,0x47) | 0x1160 (4448) | Rebel Alliance | L310 | static |
| cmd 0x6a at (0x1b8,0x47) | 0x1161 (4449) | Galactic Empire | L309 | static |
| cmd 0x6c at (0x185,0x85) | 0x100f (4111) | Standard Galaxy | L311 | static |
| cmd 0x6d at (0x1b8,0x85) | 0x100e (4110) | Large Galaxy | L312 | static |
| cmd 0x6e at (0x1eb,0x85) | 0x100d (4109) | Huge Galaxy | L313 | static |
| cmd 0x68 / 0x69 at (0x8b,0x139) / (0x15a,0x139) | none | (labels 0x1109 "Connect To Game", 0x1108 "Setup Game") | L307-308 | tooltip arg 0; label only |

### 1c. Game Options (`FUN_00407180`; see `ghidra/notes/game-options.md`; module TEXTCOMM)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| Save row ×6, cmd 0x6f..0x74, (0x22, y), 0x2a×0x14 | 0x1010 (4112) | Save Game | FUN_00407180 L90 | static, one per slot |
| Load row ×6, cmd 0x79..0x7e (per notes) | 0x1011 (4113) | Load Game | L208 | guess on the control: the decompile lost the second button's constructor in the loop and shows the call on the reused `piVar8` |
| Music volume slider +0xdc, cmd 0x8d (`FUN_00604570` at 0x188,0x82) | 0x1012 (4114) | Music Volume | L259 | static |
| Effects volume slider +0xe0, cmd 0x8e (0x188,0xbe) | 0x1013 (4115) | Sound Effects Volume | L236 | static |
| Restart cmd 0x91 at (0x4c,0x17d) | 0x1014 (4116) | "Restart the Game " | L273 | static; trailing space in the original |
| Return cmd 0x90 at (0xa2,0x17e) | 0x1015 (4117) / 0x1019 (4121) | Return to the Command Center / Disconnect from Opponent | L319 | chosen at create: 0x1019 when state `+0x9c` is 4 or 5 (multiplayer) |
| Exit cmd 0x8f at (0xf8,0x17d) | 0x1018 (4120) | Exit | L337 | static |

### 1d. REBDLOG message boxes (string module 0xc, inferred REBDLOG)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| Yes/No box: Yes cmd 0x65 at (0x8a,0x87) | 0x1300 (4864) | Yes | `FUN_00416e90` via `FUN_00606c90` (box `FUN_00416c50` / `FUN_00401110`) | static |
| Yes/No box: No cmd 0x66 at (0xe5,0x87) | 0x1301 (4865) | No | FUN_00416e90 | static |
| One-button box: cmd 0x65 at (0xb0,0x86) | button-label id from the caller (`+0x120`, 5th argument of `FUN_00417020`) | 0x190d Resume (`FUN_00415e60`), 0x190e To Cockpit (`FUN_00413560`), 0x190f Acknowledged (`FUN_00410dd0`, `FUN_00410c70`, `FUN_0040ec60`), 0x1300 Yes (save succeeded/failed, `FUN_00408170` / `FUN_00408240`) | `FUN_00417150` | static per box; the argument mapping is inferred from the 16-argument `FUN_00416f80` call |

## 2. Command center chrome and cockpit

### 2a. Finder / tool buttons (`FUN_004286b0`, called from `FUN_00422ce0` L248 with the TEXTSTRA handle; buttons built in `FUN_00427270`)

Positions are A / E from `FUN_00427270` (A lines 73-160, E lines 395-490).

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| +0x134 cmd 0x12d, A (0x6a,0x198) / E (0x8f,0x1b2) | 0x1501 (5377) | System Finder | FUN_004286b0 L13 | static |
| +0x138 cmd 0x12e, A (0x9d,0x197) / E (199,0x1b2) | 0x1502 (5378) | Fleet Finder | L19 | static |
| +0x13c cmd 0x12f, A (0x102,0x195) / E (0x19c,0x1b1) | 0x1503 (5379) | Personnel Finder | L25 | static |
| +0x140 cmd 0x130, A (0xd1,0x195) / E (0xfd,0x1b1) | 0x1504 (5380) | Troop Finder | L31 | static |
| +0x144 cmd 0x131, A (0x18a,0x195) / E (0x1d1,0x1b2) | 0x1505 (5381) | Encyclopedia | L37 | static |
| +0x148 cmd 0x132, A (0x1be,0x196) / E (0x207,0x1b2) | 0x1506 (5382) | Galactic Information Display | L43 | static |
| +0x14c cmd 0x133, A (3,0x163) / E (0x4f,0xc0) | 0x1500 (5376) | Game Controls | L49 | static |
| +0x248 legend button cmd 0x134, 30×30 at (55,50) (gid-markers.md) | 0x173c (5948) | Legend | L56 | static |

### 2b. Top-bar rect tools (`FUN_00422ce0` L222-244, `FUN_00600b30`, TEXTSTRA)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| rect id 0x14f, A (0x67,0x15)-(0xb9,0x21), E (0x1f3,0x13)-(0x244,0x1f) | 0x1513 (5395) | Game Speed Control | FUN_00422ce0 L222 | static rect |
| rect id 0x14c, A (0xf0,0x10)-(0x14a,0x21), E (0x95,0x12)-(0xf1,0x25) | 0x1510 (5392) | Raw Materials Monitor | L229 | static rect |
| rect id 0x14d, A (0x14f,0x10)-(0x1a9,0x21), E (0xf8,0x12)-(0x154,0x25) | 0x1511 (5393) | Refined Materials Monitor | L237 | static rect |
| rect id 0x14e, A (0x1ae,0x10)-(0x208,0x21), E (0x159,0x12)-(0x1b5,0x25) | 0x1512 (5394) | Maintenance Monitor | L244 | static rect |

### 2c. Cockpit message-category buttons (`FUN_00427270`, `FUN_00603870`, TEXTSTRA)

Two layouts. A is a column at x=3 (lines 199-382); E is a column at x=0x263
(lines 525-711). The buttons are 0x1b×0x16 and keep the same commands.

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| +0x168 cmd 0x136, y A 0x6d / E 0x6e | 0x8011 (32785) | Popular Support Messages | L313 / L640 | static |
| +0x164 cmd 0x137, y 0x86 / 0x87 | 0x8017 (32791) | Fleet Messages | L290 / L617 | static |
| +0x170 cmd 0x138, y 0x9f / 0xa0 | 0x8014 (32788) | Mission Messages | L359 / L686 | static |
| +0x174 cmd 0x139, y 0xb8 / 0xb9 | 0x8012 (32786) | Resource Messages | L382 / L711 | static |
| +0x16c cmd 0x13a, y 0xd1 / 0xd2 | 0x8013 (32787) | Manufacturing Messages | L336 / L663 | static |
| +0x160 cmd 0x13b, y 0xea / 0xeb | 0x8016 (32790) | Defense Messages | L267 / L594 | static |
| +0x15c cmd 0x13c, y 0x103 / 0x104 | 0x8018 (32792) | Conflict Messages | L244 / L571 | static |
| +0x154 cmd 0x13d, y 0x11c / 0x11d | 0x8032 (32818) | Advice Messages | L199 / L525 | static |
| +0x158 cmd 0x13e, y 0x135 / 0x136 | 0x8015 (32789) | Chat Messages | L221 / L548 | static |

## 3. Finders (all TEXTSTRA)

### 3a. System Finder (`FUN_00460270`, create slot +0x38)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| tab cmd 1 at (0,0) on +0x144 | 0x1867 (6247) | All Systems | FUN_00460270 L194 | static |
| tab cmd 2 at (0x34,0) | 0x1868 (6248) | Rebel Systems | L211 | static |
| tab cmd 3 at (0x68,0) | 0x1869 (6249) | Imperial Systems | L228 | static |
| tab cmd 4 at (0x9c,0) | 0x1870 (6256) | Neutral Systems | L245 | static |
| tab cmd 5 at (0xd0,0) | 0x1871 (6257) | Unexplored Systems | L262 | static |
| Close cmd 200, A (0x1ab,0x19) / E (0x1aa,0x15) | 0x1954 (6484) | Close | L267/L274 via `FUN_00606c90` | static |
| Display cmd 0xc9 | 0x1953 (6483) | Display | L280 via `FUN_00606c90` | static |

### 3b. Fleet Finder / Ship Finder (`FUN_00461960`; list refresh `FUN_00462be0`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| side tab cmd 1 at (0,0) on +0x14c | 0x1882 (6274) → 0x1887 (6279) | All Fleets → All Ships | FUN_00461960 L253; update FUN_00462be0 L278/L288 | **runtime 0x40c**: in Ship mode (`+0x148 != 1`) it becomes "All Ships" |
| side tab cmd 2 at (0x34,0) | 0x1883 (6275) → 0x1888 (6280) | Alliance Fleets → Alliance Ships | L269; update L281/L291 | runtime |
| side tab cmd 3 at (0x68,0) | 0x1884 (6276) → 0x1889 (6281) | Imperial Fleets → Imperial Ships | L285; update L295 | runtime |
| mode tab cmd 2 at (0,0) | 0x1885 (6277) | Ship Finder | L447 | static |
| mode tab cmd 1 at (0,0x36) | 0x1880 (6272) | Fleet Finder | L483 | static |
| Close cmd 200, A (0x1a7,0x19) / E (0x1aa,0x15) | 0x1954 (6484) | Close | L290/L297 via `FUN_00606c90` | static |
| Display cmd 0xc9 | 0x1953 (6483) | Display | L303 via `FUN_00606c90` | static |

### 3c. Personnel Finder (`FUN_004637a0`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| Display cmd 0xc9 | 0x1953 (6483) | Display | FUN_004637a0 L362 | static |
| type tab cmd 3 at (0,0) on +0x148 | 0x1899 (6297) | Characters | L386 | static |
| type tab cmd 2 at (0,0x36) | 0x1898 (6296) | SpecForces | L411 | static |
| side tab cmd 1 at (0,0) on +0x144 | 0x1893 (6291) | Alliance Personnel | L443 | static |
| side tab cmd 2 at (0x34,0) | 0x1894 (6292) | Imperial Personnel | L459 | static |
| Close cmd 200 | 0x1954 (6484) | Close | L475 via `FUN_00606c90` | static |

### 3d. Troop Finder (`FUN_0046ce40`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| side tab cmd 1 at (0,0) on +0x144 | 0x1950 (6480) | Alliance Troops | FUN_0046ce40 L243 | static |
| side tab cmd 2 at (0x34,0) | 0x1951 (6481) | Imperial Troops | L260 | static |
| Close cmd 200, A (0x1ab,0x19) / E (0x1aa,0x15) | 0x1954 (6484) | Close | L265/L272 via `FUN_00606c90` | static |
| Display cmd 0xc9 | 0x1953 (6483) | Display | L278 via `FUN_00606c90` | static |

## 4. Message Index (`FUN_004665f0` layout; runtime `FUN_00468fb0`, `FUN_004697b0`, `FUN_00467f10`, `FUN_00469de0`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| category cmd 0x79 at (0,0) on +0x160 | 0x8010 (32784) | All Messages | FUN_004665f0 L294 | re-set on mode-1 entry (FUN_00468fb0 L82) |
| cmd 0x7a at (0x26,0) | 0x8011 (32785) | Popular Support Messages | L318 | re-set L86 |
| cmd 0x7b at (0x4c,0) | 0x8017 (32791) | Fleet Messages | L342 | static |
| cmd 0x7c at (0x72,0) | 0x8014 (32788) | Mission Messages | L366 | re-set L98 |
| cmd 0x7d at (0x97,0) | 0x8012 (32786) | Resource Messages | L382 | re-set L90 |
| cmd 0x7e at (0xbd,0) | 0x8013 (32787) | Manufacturing Messages | L398 | re-set L94 |
| cmd 0x7f at (0xe3,0) | 0x8016 (32790) | Defense Messages | L414 | re-set L102 |
| cmd 0x80 at (0x107,0) | 0x8018 (32792) | Conflict Messages | L430 | static |
| cmd 0x81 at (0x12d,0) | 0x8015 (32789) | Chat Messages | L446 | re-set L107 |
| cmd 0x82 at (0x152,0) | 0x8032 (32818) | Advice Messages | L470 | static |
| Close cmd 0x28 | 0x1954 (6484) | Close | L508 | static |
| cmd 0x65 Display Message | 0x8020 (32800) ↔ 0x8019 (32793) | Display Message ↔ Message Index | L543; FUN_00468fb0 L113 (mode 1), L157 / L211 (modes 2/3) | **runtime**: "Message Index" while one message is open |
| cmd 0x66 alert toggle | 0x8005 (32773) ↔ 0x8006 (32774) | Post Messages Silently ↔ Post Messages with Alert | L573; FUN_004697b0 L145/L155, FUN_00467f10 L78/L87 | **runtime**: follows bit 2 of the category's flags (`FUN_0041d250` / `FUN_0041d270`) |
| cmd 0x67 | 0x8030 (32816) ↔ 0x8031 (32817) | Display ↔ "Encylopedia" (sic) | L605; FUN_004697b0 L129, FUN_00467f10 L231/L294, FUN_00469de0 L83/L101/L125 | **runtime**: 0x8030 when the message target is type 3 or 5, 0x8031 when type 4 |
| cmd 0x67 (alternate chat layout) | 0x8022 (32802) | Compose Chat Message | L640 | static; built instead of the 0x8030 button in the chat layout |
| cmd 0x90 at (0x11a,0x57) | 0x8008 (32776) | Select All | L666 | static |
| cmd 0x91 at (0x154,0x57) | 0x8009 (32777) | Delete Selected Messages | L688 | static |
| cmd 0x97 display toggle at (0x160,0xff) | 0x8030 (32816) | Display | L787 | static |

## 5. Encyclopedia (`FUN_0045ddc0`, TEXTSTRA)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| Close cmd 0xfb at (0x1aa,0x15) | 0x1954 (6484) | Close | FUN_0045ddc0 L202 | static |
| cmd 0x68 at (0,0x36) on +0x11c | 0x1859 (6233) | Index | L297 | static |
| cmd 0x67 at (0,0) on +0x11c | 0x1860 (6240) | Display Topic | L328 | static |
| db tab cmd 0x6f at (0,0) on +0x124 | 0x1850 (6224) | All Databases | L358 | static |
| cmd 0x70 at (0x34,0) | 0x1855 (6229) | System Database | L432 | static |
| cmd 0x71 at (0x68,0) | 0x1854 (6228) | Ship Database | L418 | static |
| cmd 0x72 at (0x9c,0) | 0x1852 (6226) | Facilities Database | L388 | static |
| cmd 0x73 at (0xd0,0) | 0x1851 (6225) | Missions Database | L373 | static |
| cmd 0x74 at (0x104,0) | 0x1856 (6230) | Troop Database | L447 | static |
| cmd 0x75 at (0x138,0) | 0x1853 (6227) | Personnel Database | L403 | static |
| +0x140 cmd 0x84 at (0x17c,0xe) | 0x1857 (6231) | Next Topic | L530 | static |
| +0x144 cmd 0x83 at (0x1c,0xe) | 0x1858 (6232) | Previous Topic | L545 | static |

## 6. Status and summary windows (TEXTSTRA)

### 6a. Status window (`FUN_00443020`, slot 16; module 2 from `FUN_00442d70`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| cmd 0x66 at (0x102,0xda) | 0x1505 (5381) | Encyclopedia | FUN_00443020 L17 via `FUN_00606c90` | static |
| cmd 0x65 at (0x144,0xda) | 0x1954 (6484) | Close | L36 via `FUN_00606c90` | static |

### 6b. Battle Alert (`FUN_0044f860`, class 0x659d28)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| tab cmd 1 on +0x134 | 0x1922 (6434) | Battle Summary | FUN_0044f860 L264 | static |
| tab cmd 2 | 0x1923 (6435) | Alliance Forces | L291 | static |
| tab cmd 3 | 0x1924 (6436) | Imperial Forces | L318 | static |
| tab cmd 4 | 0x1925 (6437) | System Assets | L346 | static |
| +0x120 cmd 0x65 at (0x92,0x128) | 0x1926 (6438) | Simulate Results | L379 | static |
| +0x124 cmd 0x66 at (0x118,0x128) | 0x1927 (6439) | Take Command | L409 | static |
| +0x128 cmd 0x67 at (0xc,0x128) | 0x194a (6474) | Retreat | L438 | static |

### 6c. Battle Summary (`FUN_0046f360`, vtable 0x65a300)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| tab cmd 0x6f on +0x15c | 0x1922 (6434) | Battle Summary | FUN_0046f360 L448 | static |
| tab cmd 0x70 | 0x1937 (6455) | Alliance Forces | L476 | static |
| tab cmd 0x71 | 0x1938 (6456) | Imperial Forces | L504 | static |
| tab cmd 0x72 | 0x1939 (6457) / 0x1931 (6449) | Display / Personnel | L532 / L561 | **uncertain**: the decompile builds two 0x72 buttons in a row; it may be a faction or condition branch |
| tab cmd 0x65 on +0x160 | 0x1928 (6440) | Capital Ships | L653 | static |
| tab cmd 0x66 at (0x31,0) | 0x1929 (6441) | Fighter Squadrons | L678 | static |
| tab cmd 0x67 at (0x62,0) | 0x1930 (6448) | Trooper Regiments | L703 | static |
| tab cmd 0x68 at (0x93,0) | 0x1931 (6449) | Personnel | L729 | static |

### 6d. Assault / bombardment Summary (`FUN_0044c630`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| tab cmd 0x65 on +0x15c | 0x1944 (6468) | Summary | FUN_0044c630 L441 | static |
| tab cmd 0x66 | 0x1937 (6455) or 0x1938 (6456) | Alliance Forces / Imperial Forces | L489 | chosen at create by report `+0x6c` (1 → Alliance, 2 → Imperial, else none) |
| tab cmd 0x67 | 0x1943 (6467) | System Forces | L546 | static |
| tab cmd 0x68 | 0x1939 (6457) | Display | L576 | static |
| tab cmd 0x97 at (0,0) on +0x160 | 0x1928 (6440) | Capital Ships | L620 | static |
| tab cmd 0x98 at (0x3e,0) | 0x1929 (6441) | Fighter Squadrons | L644 | static |
| tab cmd 0x99 at (0x7c,0) | 0x1835 (6197) | Manufacturing | L668 | static |
| tab cmd 0x9a at (0xba,0) | 0x1946 (6470) | Defense Facilities | L684 | static |
| tab cmd 0x9b at (0xf8,0) | 0x1930 (6448) | Trooper Regiments | L708 | static |
| tab cmd 0x9c at (0x138,0) | 0x1931 (6449) | Personnel | L733 | static |

## 7. Galaxy windows (TEXTSTRA)

### 7a. Sector window (`FUN_00459e30`, vtable slot 14): bars for every system

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| energy bar (`FUN_004acec0`, item id `(sys<<6)\|1`) | 0x1822 (6178) | Energy Consumption | FUN_00459e30 L298 | **runtime** `FUN_0045c240`: "Energy Consumption" + 0x1824 " " + n + 0x1825 "/" + m (used/capacity; order of n and m is a guess) |
| raw-materials bar (`(sys<<6)\|2`) | 0x1823 (6179) | Raw Materials | L319 | **runtime** `FUN_0045c450`: same format |
| popular-support bar (`(sys<<6)\|3`) | 0x1821 (6177) | Popular Support | L341 | static |

### 7b. Fleet window (`FUN_004a4b10`): two tab strips (own +0x158, other +0x15c)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| cmd 0x66 at (1,0) | 0x3823 (14371) | Ships | FUN_004a4b10 L187/L202 | static, both strips |
| cmd 0x67 at (0x21,0) | 0x3820 (14368) | Fighter Squadrons | L218/L234 | static |
| cmd 0x68 at (0x41,0) | 0x3821 (14369) | Troop Regiments | L251/L266 | static |
| cmd 0x69 at (0x61,0) | 0x3822 (14370) | Personnel | L282/L298 | static |

### 7c. System Defenses (type 10, `FUN_004a8790`): tabs on +0x160

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| cmd 5 at (0xac,0) | 0x1740 (5952) | Planetary Batteries | FUN_004a8790 L182 | static |
| cmd 4 at (0x88,0) | 0x1741 (5953) | Planetary Shields | L198 | static |
| cmd 3 at (100,0) | 0x1742 (5954) | Fighter Squadrons | L212 | static |
| cmd 2 at (0x40,0) | 0x1743 (5955) | Trooper Regiments | L226 | static |
| cmd 1 at (0x1c,0) | 0x1744 (5956) | Personnel | L240 | static |

The title-bar buttons (`0x277c` close, `0x280d` cmd 0xc9, `0x27e1` restore
cmd 0xca) pass tooltip 0.

### 7d. Missions window (type 11, `FUN_0049f540`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| tab cmd 0x17 at (0,0) on +0x15c | 0x8500 (34048) | Agents | FUN_0049f540 L288 | static |
| tab cmd 0x18 at (0x3d,0) | 0x8501 (34049) | Decoys | L305 | static |

### 7e. Manufacturing window (type 9, `FUN_00455060`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| cmd 0x67 at (0,0) on +0x16c | 0x1835 (6197) | Manufacturing | FUN_00455060 L535 | static |
| cmd 0x68 at (0x27,0) | 0x1828 (6184) | Shipyards | L624 | static |
| cmd 0x69 at (0x4d,0) | 0x1830 (6192) | Training Facilities | L652 | static |
| cmd 0x6a at (0x73,0) | 0x1832 (6194) | Construction Yards | L680 | static |
| cmd 0x6b at (0x98,0) | 0x1827 (6183) | Refineries | L596 | static |
| cmd 0x6c at (0xbe,0) | 0x1826 (6182) | Mines | L566 | static |

### 7f. Build Selection child (`Class_00658db8`, module 2 from `FUN_00437df0`; REBDLOG has different texts for some of these ids, so the module matters)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| quantity edit box +0x128 (`FUN_00604cf0` at (0x8d,0xc4) 0x2d×0x11) | 0x190c (6412) | Edit Number | FUN_00437f80 L210 | static |
| cmd 0x6c at (0x4f,0x5a) | 0x1906 (6406) | Items to Build | FUN_00438620 L8 via `FUN_00606c90` | static (REBDLOG would read "Item Build") |
| cmd 0x65 at (0x49,0xe0) | 0x1909 (6409) | Done | L9 | static |
| cmd 0x66 at (0x8d,0xe0) | 0x190a (6410) | Cancel | L15 | static |
| cmd 0x67 at (5,0xe0) | 0x190b (6411) | Encyclopedia | L16 | static (REBDLOG would read "Lookup") |
| cmd 0x69 at (0xbd,0xc4) | 0x1907 (6407) | Increase | L17 | static |
| cmd 0x6a at (0xbd,0xcd) | 0x1908 (6408) | Decrease | L18 | static |
| close cmd 100 at (0xc1,3) (`FUN_00606e10`) | 0x1903 (6403) | Close | L7 | static |

A0 `.artifacts/a0-vm/build-selection/tip-*` (2026-10-31, Kimanan): every
tip shows at the pointer plus (0, 21). The text is Close, Cancel, Increase,
Decrease, Edit Number, Encyclopedia or Items to Build, each 4 pixels from
the left border with its capitals 5 pixels down. The disabled Done shows
no tip.

### 7g. Move-order confirmation (`FUN_0044f180`, slot +0x38)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| cmd 0x14 at (0x163,0xf4) | 0x1300 (4864) | Yes | FUN_0044f180 L130 | static |
| cmd 0x15 at (0x163,0x119) | 0x1301 (4865) | No | L148 | static |

## 8. Mission dialog (`FUN_0046a9c0`, TEXTSTRA)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| cmd 0x68 at (0x65,0xae) 0x41×0x12 | 0x8510 (34064) | Missions | FUN_0046a9c0 L263 | static |
| cmd 0x65 at (0xaa,0x140) | 0x8503 (34051) | Cancel | L281 | static |
| cmd 0x66 at (0x66,0x140) | 0x8502 (34050) | Begin Mission | L299 | static |
| cmd 0x67 at (0x21,0x140) | 0x8031 (32817) | "Encylopedia" (sic) | L317 | static |
| +0x128 selected-mission control | 0x8520 (34080) | Selected Mission | L434 | static |
| cmd 0xca at (0x78,0x88) 16×16 | 0x8505 (34053) | Use Selected Agents as Decoys | L544 | static |
| cmd 0xcb at (0x78,0xdd) 16×16 | 0x8506 (34054) | Use Selected Decoys as Agents | L565 | static |
| close cmd 100 at (0xf2,3) | none | | L240 | no tooltip |

## 9. Tactical battle (TEXTTACT; `Class_0066dbc8_vt4c(ctrl, label, tooltip)` unless stated)

### 9a. Main tactical HUD (`Class_0066bb00`, ctor `FUN_0059a8d0`; command router `FUN_0059b130`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| Task Forces header cmd 0xad01 at (9,1) | 0xad91 (44433) | Task Forces | FUN_0059be80 L44 | static |
| Task-force buttons ×8, cmd 0xad02..0xad09, x = 0x3c + 0x1d·i, y 2, label 0xad81+i ("1".."8") | 0xad92 (44434) → 0xad93..0xad9a (44435-44442) | Unassigned Task Force → Task Force 1..8 | L69; updates FUN_0059b550, FUN_0059b450 | **runtime**: "Task Force N" (array `this+0xd8` = `DAT_006bc3a0`, `3a0, 39c, 3c8, 3c0, 3d0, 3cc, 3b4, 3ac`) once assigned; back to "Unassigned Task Force" (`DAT_006bc398`) when empty |
| Fighter header cmd 0xad0a at (0x128,1) | 0xad9b (44443) | Fighter Squadrons | FUN_0059c030 L40 | static |
| Fighter-group buttons ×4, cmd 0xad0b..0xad0e, x = 0x15b + 0x1d·i | 0xad9d..0xada0 (44445-44448) ↔ 0xad9c (44444) | Red/Blue/Green/Gold Fighter Group ↔ Inactive Fighter Group | L61; update FUN_0059b690 | **runtime**: array `this+0xf8` = `DAT_006bc434, 2c4, 2f4, 328`; "Inactive Fighter Group" (`DAT_006bc2b4`) when the group is empty |
| Alliance highlight cmd 0xad16 at (0x203,0x131) | 0xada6 (44454) ↔ 0xada7 (44455) | Highlight Alliance Ships ↔ Dim Alliance Ships | FUN_0059c4d0 L33; update FUN_0059b130 case 0xad16 | **runtime**, toggles with the button's state bit 4 |
| Empire highlight cmd 0xad17 at (0x1e3,0x131) | 0xada8 (44456) ↔ 0xada9 (44457) | Highlight Empire Ships ↔ Dim Empire Ships | L49; update case 0xad17 | runtime |
| Nav point 1..4, cmd 0xad12..0xad15 at x 0x1e5/0x20c/0x233/0x25a, y 0x110 | 0xada2..0xada5 (44450-44453) | Navigation Point Set 1..4 | FUN_0059c260 L30-75 | static |
| Game Options cmd 0xad18 at (0x260,0x135) | 0xadaa (44458) | Game Options | FUN_0059bd30 L27 | static |
| Pause cmd 0xad22 at (0x231,0x134) | 0xadb8 (44472) ↔ 0xadb7 (44471) | Resume Battle ↔ Pause Battle | L42; update FUN_0059b130 case 0xad22 | **runtime**: "Pause Battle" while running, "Resume Battle" while paused |
| Zoom In cmd 0xad1b at (0x1eb,0x157) | 0xadb6 (44470) | Zoom In | FUN_0059c650 L30 | static |
| Zoom Out cmd 0xad1c at (0x25f,0x157) | 0xadb5 (44469) | Zoom Out | L43 | static |
| Rotate Left cmd 0xad1d at (0x1f1,0x17d) | 0xadb4 (44468) | Rotate Left | L56 | static |
| Rotate Right cmd 0xad1e at (0x246,0x17d) | 0xadb3 (44467) | Rotate Right | L69 | static |
| Tilt Up cmd 0xad1f at (0x21b,0x152) | 0xadb2 (44466) | Tilt Up | L82 | static |
| Tilt Down cmd 0xad20 at (0x21c,0x1a8) | 0xadb1 (44465) | Tilt Down | L95 | static |
| Remember camera cmd 0xad19 at (0x252,0x1bc) | 0xadab (44459) | Remember Camera Position | L108 | static |
| Reset view cmd 0xad1a at (0x1e6,0x1bc) | 0xadad (44461) | Reset Camera View | L121 | static |
| Reset subject cmd 0xad21 at (0x225,0x187) | 0xadb0 (44464) | Reset Camera Subject | L134 | static |

### 9b. Battle control panel (`Class_0066d538_vt48` = `FUN_005dab90`; commands `Class_0066d538_vt54` = `FUN_005dae10`)

The third arguments come from `objdump` push operands.

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| cmd 0xdd01 at (0x12,0x13) | 0xddba (56762) | Withdraw From Battle | FUN_005dab90 (push 0x6bcd2c) | static |
| cmd 0xdd02 at (0xe,0x5d) | 0xddbb (56763) | Get Immediate Result | (push 0x6bcd4c) | static |
| cmd 0xdd03 at (0x57,0x5d) | 0xddbc (56764) ↔ 0xdded (56813) | "Observe Battle / TAKE COMMAND" ↔ "Take Command / OBSERVE BATTLE" | (push 0x6bcd24); update FUN_005dae10 L60/L64 | **runtime**, toggles with the button state |
| cmd 0xdd04 at (0x17,0xb8) | 0xddbd (56765) | Game Options | (push 0x6bccb8; the decompile shows 2 args) | static |
| cmd 0xdd05 at (0x61,0xb8) | 0xddbe (56766) | Close | (push 0x6bcc88) | static |

### 9c. Maneuvers panel (`Class_0066d5a0_vt48` = `FUN_005dcc70`; commands `Class_0066d5a0_vt54` = `FUN_005dd080`, `FUN_005dd3a0`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| cmd 0xdd10 at (0xd,0x9f) | 0xddd0 (56784) | Left Hook | FUN_005dcc70 | static |
| cmd 0xdd11 at (0x2b,0x9f) | 0xddd1 (56785) | Right Hook | | static |
| cmd 0xdd12 at (0xd,0x81) | 0xddd2 (56786) | Hammer | | static |
| cmd 0xdd13 at (0x2b,0x81) | 0xddd3 (56787) | Anvil | | static |
| cmd 0xdd14 at (0x4b,0x90) | 0xddd4 (56788) | Stop | | static |
| cmd 0xdd20 at (0x6f,0x84) 0x18×0x34 | 0xddcf (56783) ↔ 0xddce (56782) | "Surround / STAND OFF" ↔ "Stand Off / SURROUND" | create; update FUN_005dd080 case 0xdd20, FUN_005dd3a0 L67/L71 | **runtime**, toggles with the button state |
| cmd 0xdd01 at (0x42,0xca) | 0xddb9 (56761) | Assign Maneuver / Tactics | (push 0x6bcf70; the decompile shows 2 args) | static |
| cmd 0xdd02 at (0x6a,0xca) | 0xdda1 (56737) | Cancel | | static |

### 9d. Fighter orders panel (`Class_0066d608_vt48` = `FUN_005df110`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| cmd 0xdd03 at (0x12,0x81) | 0xdde5 (56805) | Attack Capital Ships | FUN_005df110 L53 | static |
| cmd 0xdd05 at (0x56,0x81) | 0xdde2 (56802) | Recover | L79 | static |
| cmd 0xdd06 at (0x56,0x9f) | 0xdde6 (56806) | Attack Death Star | L98 | static; hidden unless the Death Star is present |
| cmd 0xdd04 at (0x12,0x9f) | 0xddeb (56811) | Attack Fighters | L122 | static |
| cmd 0xdd01 at (0x42,0xca) | 0xdda0 (56736) | Assign Mission | L134 | static |
| cmd 0xdd02 at (0x6a,0xca) | 0xdda1 (56737) | Cancel | L142 | static |

### 9e. Fighter group panel (`Class_0066d670_vt48` = `FUN_005e1550`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| cmd 0xdd02 at (0xc,0xd2) | 0xddcd (56781) | Maneuvers & Tactics | FUN_005e1550 L30 | static |
| cmd 0xdd01 at (0x50,0xd2) | 0xdd8c (56716) | Missions | L38 | static |

### 9f. Capital ship panel, ship in a task force (`Class_0066d6e0_vt48` = `FUN_005e4110`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| hull bar `param_1[0x28]` (`FUN_0060e150` at (0x22,0x51) 0x25×10) | 0xdd9a (56730) | Hull Integrity | FUN_005e4110 L34 | static (`FUN_00600a40`) |
| shield bar `param_1[0x29]` at (0x67,0x51) | 0xdd9b (56731) | Shield Strength | L51 | static |
| cmd 0xdd03 at (7,10) 0xb×0x35 | 0xdd81 (56705) | Previous Capital Ship in Task Force | L63 | static |
| cmd 0xdd04 at (0x84,10) | 0xdd82 (56706) | Next Capital Ship in Task Force | L71 | static |
| cmd 0xdd01 at (0x50,0xd2) | 0xdd8c (56716) | Missions | L80 (push 0x6bd658) | static |
| cmd 0xdd02 at (0xc,0xd2) | 0xddcd (56781) | Maneuvers & Tactics | L88 | static |
| rect id 0xdd08 (10,0x67)-(0x20,0x76) | 0xddd5 (56789) | Shield Recharge | L89 (`FUN_00600b30`) | static rect (power-allocation bars) |
| rect id 0xdd09 (0x25,0x67)-(0x3b,0x76) | 0xddd6 (56790) | Weapon Recharge | L90 | static rect |
| rect id 0xdd0a (0x40,0x67)-(0x56,0x76) | 0xddd7 (56791) | Tractor Beam Power | L91 | static rect |
| rect id 0xdd0b (0x5b,0x67)-(0x71,0x76) | 0xddd8 (56792) | Engines | L92 | static rect |
| rect id 0xdd0c (0x76,0x67)-(0x8c,0x76) | 0xddd9 (56793) | Hyperspace Engines | L93 | static rect |

### 9g. Capital ship panel, ship without a task force (`Class_0066d750_vt48` = `FUN_005e7030`)

Same layout as 9f, with these differences:

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| hull / shield bars | 0xdd9a / 0xdd9b | Hull Integrity / Shield Strength | FUN_005e7030 L37/L53 | static |
| cmd 0xdd02 at (7,10) | 0xdd84 (56708) | Previous Capital Ship | L65 | static |
| cmd 0xdd03 at (0x84,10) | 0xdd85 (56709) | Next Capital Ship | L73 | static |
| cmd 0xdd01 at (0x50,0xd2) | 0xdd8c (56716) | Missions | L79 (push 0x6bd868) | static |
| cmd 0xdd07 at (0xc,0xd2) | 0xddcd (56781) | Maneuvers & Tactics | L88 | static |
| rect ids 0xdd08..0xdd0c | 0xddd5..0xddd9 | Shield Recharge, Weapon Recharge, Tractor Beam Power, Engines, Hyperspace Engines | L89-93 | static rects, same rectangles as 9f |

### 9h. Withdraw confirmation panel (`Class_0066dc30_vt48` = `FUN_005f0f40`)

| Control | String id | Text | Source | Notes |
|---|---|---|---|---|
| cmd 0xdd01 at (0x42,0xca) | 0xddc2 (56770) | Confirm Withdraw | FUN_005f0f40 L30 | static |
| cmd 0xdd02 at (0x6a,0xca) | 0xdda1 (56737) | Cancel | L38 | static |

---

## Runtime updates (TTM_UPDATETIPTEXT) in one list

| Updater | Window | What changes |
|---|---|---|
| `FUN_0045c240`, `FUN_0045c450` | Sector window | energy and raw-material bars: "label n/m" |
| `FUN_00462be0` | Fleet Finder | side tabs: Fleets ↔ Ships wording |
| `FUN_00468fb0` | Message Index | on mode change: category tooltips reset; 0x65 Display Message ↔ Message Index |
| `FUN_004697b0`, `FUN_00467f10` | Message Index | 0x66 Silently ↔ with Alert; 0x67 Display ↔ Encylopedia |
| `FUN_00469de0` | Message Index | 0x67 Display / Encylopedia for the opened message |
| `FUN_0059b130` | Tactical HUD | Highlight ↔ Dim (A and E); Pause ↔ Resume |
| `FUN_0059b450`, `FUN_0059b550` | Tactical HUD | task-force buttons: Unassigned ↔ Task Force N |
| `FUN_0059b690` | Tactical HUD | fighter groups: colour group ↔ Inactive |
| `FUN_005dae10` | Battle control | Observe ↔ Take Command |
| `FUN_005dd080`, `FUN_005dd3a0` | Maneuvers | Surround ↔ Stand Off |

## Files used

- Corpus: `~/Games/rebexe-decompile/decompiled-typed/` (every function above).
- EXE: `/home/retromax/Games/Heroic/Star Wars - Rebellion/REBEXE.EXE` (`.rdata`
  module words, `.bss` init stubs, `objdump` of `0x5dab90`, `0x5dcc70`,
  `0x5df110`, `0x5e4110`, `0x5e7030`).
- Strings: `web/data/base/textstra.json`, plus RT_STRING read from
  `web/data/base/{TEXTCOMM,TEXTTACT,REBDLOG}.DLL`.

## Disabled controls show no tip


## Answer

The Personnel Finder's command handler calls `EnableWindow(FALSE)` on the
type tab that was just chosen and `EnableWindow(TRUE)` on the other one.
Win32 shows no tooltip for a disabled tool window. The Fleet Finder's mode
handler does not enable or disable anything, so both of its tabs keep
their tips. The tab-strip class does not disable anything by itself.
Nothing deletes, hides, or re-creates a tool or button.

## Evidence

### 1. The Personnel Finder command slot: `Class_0065a130_vt48` = `FUN_00464890`

The vtable `0x65a130` is read from REBEXE.EXE. The case for strip `0x6e`
(the `+0x148` CoolTabControl), with HIWORD = the chosen button id:

```c
case 0x6e:
  if (param_3 >> 0x10 == 2) {               // SpecForces chosen
    piVar3 = (int *)FUN_00604500(3);  (**(code **)(*piVar3 + 0xc))();   // Characters: Enable
    piVar3 = (int *)FUN_00604500(2);  (**(code **)(*piVar3 + 0x10))();  // SpecForces: Disable
    FUN_00464e10(param_1[0x5c],0);
  }
  else if (param_3 >> 0x10 == 3) {          // Characters chosen
    piVar3 = (int *)FUN_00604500(3);  (**(code **)(*piVar3 + 0x10))();  // Characters: Disable
    piVar3 = (int *)FUN_00604500(2);  (**(code **)(*piVar3 + 0xc))();   // SpecForces: Enable
    FUN_00464e10(param_1[0x5d],0);
  }
```

The handler for case `100` (the side strip `+0x144`) only rebuilds the
list. It does not enable or disable anything, so the side tabs keep their
tips.

### 2. The CoolStrobeButton vtable `0x66df58` (from FUN_00602150)

The slots, read from the EXE: `+0xc` = `0x602880` and `+0x10` = `0x6028a0`.

```c
void Class_0066df58_vtc(void)  { FUN_006030f0(2); FUN_005ffe60(); }  // clear state bit 2, then enable
void Class_0066df58_vt10(void) { FUN_006030c0(2); FUN_005ffe80(); }  // set state bit 2, then disable

void FUN_005ffe60(int w) { if (!IsWindowEnabled(w->hwnd)) EnableWindow(w->hwnd,1); }
void FUN_005ffe80(int w) { if ( IsWindowEnabled(w->hwnd)) EnableWindow(w->hwnd,0); }
```

The base window vtable has the same Enable/Disable pair in the same slots.
The CoolTabControl vtable `0x66e230` has `+0xc = 0x5ffe60` and
`+0x10 = 0x5ffe80`. So `+0x10` is the generic "Disable" call.

### 3. A disabled type tab looks like a pressed tab

The create routine is `FUN_004637a0`. Each type tab gets its pressed
bitmap twice: once as its own pressed art, and once more as its disabled
art.

```c
iVar8 = FUN_00602150(.., 0,0, .., *(int*)(param_1+0x148), 3, uVar6, local_4c, ..);
...  FUN_00600a40(iVar8,&local_14 /*0x1899*/);  FUN_0060d700(iVar8,0,0);
     FUN_00603150(2,local_4c);     // state-2 (disabled) art := pressed art
```

`FUN_00602150` stores `param_9` (normal art) in `+0x94` and `param_10`
(pressed art) in `+0x98`. `FUN_00603150` case 2 writes `+0x9c`, the art
for state bit 2. So the disabled, chosen tab draws as "pressed". Nothing
on screen tells you it is disabled, which explains the missing tip.

The opening state comes from `FUN_0060d7e0(3,1)`. That call sends
`WM_COMMAND` with HIWORD 3, so the handler disables Characters from the
first frame (`personnel-finder.md`).

The side tabs (`+0x144`, buttons 1 and 2 with tips `0x1893` and `0x1894`)
get no `FUN_00603150(2, ..)` call and are never disabled.

### 4. The tab-strip class does not disable its buttons

The CoolTabControl class (`FUN_0060d590`, class string "CoolTabControl",
vtable `0x66e230`) selects a tab like this:

```c
// FUN_0060d7e0(strip, id, notify)
if (strip->cur) FUN_006030f0(4);          // clear "selected" bit 4 on the old tab
if (!FUN_00603120(4)) FUN_006030c0(4);    // set bit 4 on the new tab
strip->cur(+0x94) = btn;
if (notify) SendMessageA(owner, WM_COMMAND, btn->id<<16 | strip->id, 0);
```

`FUN_0060d700` (adding a tab) sets bit 8. It also sets state-4 art to the
pressed art when no state-4 art exists:
`if (!FUN_00603550(4)) FUN_00603150(4, FUN_006034a0(1))`. It then shows the
button.

So "selected" is only state bit 4, drawn with the pressed art. The window
stays enabled and its tooltip keeps working.

### 5. The Fleet Finder (`FUN_00461960`, vtable `0x65a090`, command slot `0x462770`)

```c
case 0xfa: FUN_004632d0(param_3 >> 0x10); return;
```

`FUN_004632d0` changes the background, the caret, and the list (it calls
`FUN_006075e0`, `FUN_00608300`, and `FUN_00462be0`). It makes no
Enable/Disable call.

The Fleet Finder's mode buttons call `FUN_00602840(0x260)` and get no
`FUN_00603150(2, ..)` call. So both mode tabs stay enabled and both show
tips. The chosen tab shows as pressed only through state bit 4.

### 6. How the tooltip is registered, and why a disabled tab gets none

`FUN_00600970` sends `TTM_ADDTOOLA` (`0x404`) with `cbSize` `0x2c` and
`uFlags` = 1 (`TTF_IDISHWND`), and without `TTF_SUBCLASS`. So the mouse
messages reach the tooltip through relaying.

A disabled child window gets no mouse input, and `WindowFromPoint` skips
disabled windows. So the mouse messages go to the strip parent, which is
not a tool, and no tip appears. This is standard Win32 behaviour and was
not traced in the EXE's relay code; the observed behaviour matches it.

## Rule for the port

> A tab button shows no tooltip while it is disabled. Two windows disable
> the current tab of their exclusive tab pair and enable the other. On
> CoolTabControl tabs, "selected" (state bit 4) alone never hides the tip.

A tab is disabled-when-current exactly when both of these hold:

1. at creation, its disabled art equals its pressed art
   (`FUN_00603150(2, pressed)`);
2. the owner's command or mode handler calls vtable `+0x10` on the chosen
   button and `+0x0c` on the others.

A scan of every `FUN_00604500(id)` followed by a `+0xc` or `+0x10` call
finds this exclusive-pair pattern in only two windows:

| Window | Strip | Buttons | Disabled when current | Evidence |
|---|---|---|---|---|
| Personnel Finder (vt `0x65a130`) | `+0x148`, control `0x6e` | 3 Characters (`0x1899`), 2 SpecForces (`0x1898`) | yes | `FUN_00464890` case `0x6e`; create `FUN_004637a0` |
| Encyclopedia (vt `0x659fa8`) | `+0x11c`, control `0x66` | `0x67` (tip `0x1860`, at 0,0), `0x68` (tip `0x1859`, at 0,`0x36`) | yes | `FUN_0045f480`: mode 1 enables `0x67` and disables `0x68`; mode 2 the reverse. Create `FUN_0045ddc0` lines 276-330 with `FUN_00603150(2,..)` |

`encyclopedia-window.md` says index mode makes `0x68` current, so mode 1
is the index. I did not check that mapping again here.

Tabs that are not disabled when current, so their tips always show:

- Personnel Finder side tabs (`+0x144`, control `100`);
- Fleet Finder mode tabs (`+0x144`, control `0xfa`) and side tabs
  (`+0x14c`, control `100`);
- Encyclopedia strip `0x6e` (`FUN_0045ddc0` line 338).

  Guess: I found no Disable call for the Encyclopedia strip, but only
  through the scan pattern above.

Other Disable calls are real "unavailable" states, not selection:

- Fleet window `FUN_004a6980` disables and hides tab `0x66` when
  `+0x14c & 8` is set. Its disabled art is a separate greyed bitmap
  (`0x28ad` and others), not the pressed art.
- Message Index `FUN_00469de0` disables `0x67`, and `FUN_0046a200`
  disables `0x96` and `0x9a`.
- The `0xdd0x` buttons in `FUN_005dafc0`, `FUN_005e16a0`, `FUN_005e4b40`
  and `FUN_005e7b10`.

A disabled button shows no tip in all of these cases as well.

A disabled tab also takes no clicks. So clicking the chosen Characters or
SpecForces tab again does nothing. That follows from Win32 and was not
observed separately.
