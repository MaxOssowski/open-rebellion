# The Planetary System Finder (window type 0x14)

Recovered 2026-10-30 from the GOG REBEXE.EXE: typed decompiles
(`~/Games/rebexe-decompile/decompiled-typed`), objdump of the exe for dropped
arguments, the vtable read straight from the PE (`0x0065a000`), STRATEGY.DLL
bitmaps (`data/base/ui/strategy-dll/BMP`) and TEXTSTRA
(`data/base/textstra.json`). Compared against `ghidra/notes/fleet-finder.md`
(class `FUN_00461750`, vtable `0x0065a090`). `hyp:` marks inference.

## Entry

- Cockpit control `0x12d` (`FUN_00427270`): Alliance `(106,408)` 27x16,
  10002/10001; Empire `(143,434)` 37x24, 10016/10015 (already drawn by the
  port, `CockpitButton::SystemFinder`).
- Galaxy view `FUN_00422ce0`: `WM_COMMAND 0x12d` -> `FUN_0042a000`, refused
  while `FUN_004fcee0() < 2`. `WM_KEYDOWN 0x71` (F2) -> `FUN_0042a000`
  directly (the key path has no `FUN_004fcee0` test). Keys `0x70..0x76` are
  ignored while any of the view's windows has `+0xb8` bit 0 (every finder,
  including this one, sets it) or while Ctrl is down. (F3 0x72 Fleet,
  F4 0x73 `FUN_0042a4d0` Troop, F5 0x74 `FUN_0042a180` Personnel, F1 0x70
  posts command 0x133.)
- Opener `FUN_0042a000` (byte-for-byte the Fleet opener `FUN_0042a0c0` with
  0x14/0x164/`FUN_00460090` in place of 0x15/0x170/`FUN_00461750`): posts
  `0x409` to the view; if no child of type `0x14` exists (`FUN_00604500`),
  allocates 0x164 bytes, `FUN_00460090(hinst,0,0,0x1d6,0x14a,view,0x14)`
  (470x330), clears `+0x50` bit `0x10000000`, shows it (`FUN_005ffce0(0)`),
  adds it to the view's list (`FUN_005f4f10`), calls slot `+0x2c(5)`.
  A second F2 while open does nothing (and is blocked by `+0xb8` anyway).

## Class

Constructor `FUN_00460090` (vtable `0x0065a000`): base `FUN_004ac120(...)`
(same base window ctor as the Fleet Finder), `FUN_004ece30` (empty id at
`+0x160`), `+0x94 = 2`, `+0x98 = 7`, `+0xb8 |= 1` (finder/modal flag),
`+0x144` tab group, `+0x148` name box, `+0x14c` list, `+0x150` galaxy view
(param 7), `+0x154` tab heading text, `+0x158` current tab (0), `+0x15c`
sorted row list `FUN_0060a790(2)` (mode 2 = name order, `_stricmp`, as the
Fleet Finder), `+0x160` chosen system id.

Destructor `FUN_004601a0` / deleting dtor `FUN_00460180` (vt +0).

Vtable `0x0065a000` vs Fleet `0x0065a090` (36 slots read from the PE):

| slot | System | Fleet | role |
|---|---|---|---|
| +0x00 | `FUN_00460180` | `FUN_00461860` | deleting dtor |
| +0x14 | `FUN_00461000` | `FUN_00462a50` | notifications (name box 0x407/0x408, 7 = focus) |
| +0x38 | `FUN_00460270` | `FUN_00461960` | create (layout) |
| +0x44 | `FUN_00460fe0` | `FUN_00462a30` | paint: `FUN_00601ce0(hdc)` draws the tab heading |
| +0x48 | `FUN_00460d10` | `FUN_00462770` | WM_COMMAND |
| +0x4c | `FUN_00461590` | `FUN_00463360` | WM_KEYDOWN |
| all others | identical (`0x5ffe40`, `0x607d30`, `0x606960` close at +0x30, `0x6068f0`, `0x4ac5c0`..., `0x461730`/`0x461740` stubs) | | shared base |

So the base window class (`FUN_004ac120`, slots from `0x4ac*`, `0x5ff*`,
`0x60*`) and the controls (`FUN_0060d590` tab group, `FUN_00602150` button,
`FUN_00606c90` bitmap button, `FUN_00604cf0` edit box, `FUN_00607ea0` list,
`FUN_0060a490` scroll bar, `FUN_00609650` prefix choice) are shared; the
five finder-specific functions are per class and near clones.

## Create (`FUN_00460270`, slot +0x38)

Bounds/centres in the galaxy view rect (`FUN_00606980(view+0xcc..0xd8)`),
as the Fleet Finder. Palette/instance `FUN_006037f0(7)`.

Background, composed once into one bitmap (`FUN_006075b0(bmp,1)`; single
mode, unlike the Fleet Finder's two backgrounds via `FUN_00607610`):

| part | Alliance (+0x9c = 1) | Empire (2) |
|---|---|---|
| base 470x331 | 10335 (`0x285f`) | 10336 (`0x2860`) |
| panel 400x306 at (12,13) | 10522 (`0x291a`) | 10522 (same) |
| right strip 58x330 at (412,0) | 10584 (`0x2958`) | 10588 (`0x295c`) |

(Fleet uses per-side panels 10524..10527 and strips 10586/10590.) The
decompile drops the source bitmap argument, so the order is settled by A0
(`.artifacts/a0-vm/system-finder/01-f2.png`, Empire, window at (125, 52)):
the panel lies **under** the base, which is blitted keyed on its first
pixel (index 252, blue), so the base's rounded corners cover the panel's
corners. Composed as panel, keyed base, strip, the window matches A0 in
every pixel outside the text, tabs and list (0 of the compared pixels);
composed as base then opaque panel, 86 corner pixels differ. The strip's
order against the base does not show (0 either way). The Fleet Finder's
A0 capture (`.artifacts/a0-vm/modal/01-f3.png`) shows the same corners.

Text drawn into the background:
- title TEXTSTRA `0x1865` (6245) "Planetary System Finder" at (36,14),
  width 350 (`*p = 0x15e`), `+0x2c = 1` (hyp: centred), bold
  (`FUN_00403e90(1)`), font 5, colour `0x2ffffff` (palette-relative white);
- label `0x1866` (6246) "System Name" at (36,48), font 4, same colour.

Controls:

| control | id | rect | art / text |
|---|---|---|---|
| tab group `FUN_0060d590` | 100 | (36,78) 257x41 | |
| tab 1 All | 1 | (0,0) 49x42 | 10500/10501, `0x1867` "All Systems", initially down (`FUN_0060d700(1,0)`) |
| tab 2 Rebel | 2 | (52,0) | 10502/10503, `0x1868` "Rebel Systems" |
| tab 3 Imperial | 3 | (104,0) | 10505/10506, `0x1869` "Imperial Systems" |
| tab 4 Neutral | 4 | (156,0) | 10508/10509, `0x1870` "Neutral Systems" (cyan stars) |
| tab 5 Unexplored | 5 | (208,0) | 10511/10512, `0x1871` "Unexplored Systems" (grey stars) |
| Close `FUN_00606c90` | 200 | Alliance (427,25) 10514/10515 (32x31); Empire (426,21) 10516/10517 (44x41) | `0x1954` "Close" |
| Display | 0xc9 | Alliance (427,93) 10518/10519; Empire (426,89) 10520/10521 | `0x1953` "Display" |
| name box `FUN_00604cf0` | 0xcb | (143,45) 250x18, text colour `0xffffff` | cleared (`FUN_00604f90(0)`), caret (0,0) |
| tab heading `FUN_00601880` (+0x154) | | (40,119) 283x16, `+0x2c = 3`, font 5, not bold, `0x2ffffff` | text of the current tab |
| list `FUN_00607ea0` | 0xca | (36,138) 350x165, rows 350x20; arrows 10367/10368/10365/10366 (`0x287f,0x2880,0x287d,0x287e`) | `+0xf4 = 0x30000`, `FUN_00609340(2)`, `+0xe4 = 0x24`, `+0xe8 = 5` |
| scroll bar `FUN_0060a490(..,7,0x299d,..)` | | list's height | art 10653 (not in the extracted STRATEGY BMPs) |

Both buttons play sound `0x260`. Differences from the Fleet Finder: 5 tabs
(group 257 wide vs 153, tab height 42 vs 41), no mode group / Ship Finder
button, Alliance buttons at x 427 (Fleet 423), name box 18 high (Fleet 16),
ids list 0xca / box 0xcb (Fleet 0xcc / 0xcd).

End of create: `FUN_004611d0(1, 1)` (fill tab All; 2nd arg unused, `ret 8`)
and `SetFocus(name box)`. No prefix choice at create, so on opening nothing
is chosen (A0: no white row on All).

## The list (`FUN_004611d0(tab, unused)`)

Returns at once if `tab == +0x158`. Else clears the row list
(`FUN_005f5b20`) and walks the view side's systems:
`FUN_004f31b0(side)` -> `FUN_0053ef50({0x90,0x98}, side, 1)` (families
0x90..0x97, every system in that side's view, same iterator as the Fleet
Finder's), next `FUN_004f6010` -> `FUN_0053f090`. No `+0x50` existence test
(Fleet has one). Filter (asm 0x4612b3..0x4613f7):

| tab | condition (system `+0x24` side bits 6..7, `+0x88` bit 1 = explored) |
|---|---|
| 1 All | every system |
| 2 Rebel | `(+0x24 & 0xc0) == 0x40` (explored flag not tested) |
| 3 Imperial | `(+0x24 & 0xc0) == 0x80` (explored flag not tested) |
| 4 Neutral | `(+0x24 & 0xc0) == 0xc0` **and** `+0x88 & 2` (explored) |
| 5 Unexplored | `(+0x88 & 2) == 0`, any side |

Side bits 0xc0 = the "side 3 / no side" value used for neutral systems
elsewhere (`regiment-unload.md`, `captive-escape.md`). So a system can be
on both Rebel and Unexplored (hyp: an owned system is normally explored for
its owner; for the enemy, the view object holds what the side knows).
hyp: a system with side bits 0 appears only on All (and Unexplored if
unexplored). `+0x88` bit 1 = explored per `ai-galaxy-records.md`,
`advisor-briefing.md`. hyp: the object is the side's view
(`FUN_005844e0`), so explored/side are as the viewing side knows them.

Each row: `FUN_004421d0(id)` row object, name `FUN_004f6270` (object name,
else record), inserted sorted (`FUN_005f59f0`, mode 2: case-insensitive by
name). Then `+0x158 = tab`, `FUN_0060a280` refreshes the list, the heading
`+0x154` takes the tab button's text (`FUN_00604500(tab)->+0x84`,
`FUN_00601aa0`) and its rect is invalidated. **The chosen id `+0x160` is not
cleared** (Fleet's fill clears `+0x16c`).

## Commands (`FUN_00460d10`, slot +0x48, low word = control id)

- `100` tab group: `FUN_004611d0(HIWORD = tab)`, then
  `FUN_00609650(list, name box text, 0, 0)` — the same prefix choice as the
  name box: the first row with the longest case-insensitive common prefix;
  with an empty box no row shares a letter, so the **first row** is chosen,
  highlighted and scrolled to (A0: side tab -> first row white at once). If
  it returns a row, `+0x160 = row id`; else (empty list) `+0x160` is left
  as it was (hyp: stale id, Display then opens that system's sector).
- `200` Close: slot `+0x30` (`FUN_00606960`, close/destroy).
- `0xc9` Display: only if `+0x160` has both a family byte and a non-zero low
  24 bits. `sys = FUN_004f3220(side, &+0x160)` (lookup, family 0x90..0x97
  only). If found, `FUN_00429ce0(view, sys->+0x1c id)` — the **sector**
  (`+0x1c` is the system's container; `FUN_004f3970` checks family
  0x80..0x8f) — i.e. opens, or returns the existing, sector window for the
  system's sector (`sector-window-placement.md`). Then closes the finder
  (always, even if the lookup failed). It does not select the system in
  the sector window (no call after `FUN_00429ce0`). Fleet's Display instead
  calls `FUN_00429440` (sector window + Fleet window + select).
- `0xca` list: any notification takes the selected row (`FUN_00609410`) and
  stores its id in `+0x160`; `0x309` (double click) with a row opens it as
  Display does and closes; `0x29b` (click) copies the row's name into the
  name box (`FUN_00604f90(name)`).

## Name box (`FUN_00461000`, slot +0x14)

- `7`: SetFocus(name box).
- `0x407` (Enter) from `0xcb`: if `+0x160` is not the empty id, open as
  Display (`FUN_004f3220` -> `FUN_00429ce0(sector)`) and close. With no
  chosen id Enter does nothing. (A0: "cor" + Enter -> Corellian sector.)
- `0x408` (text changed) from `0xcb`: if the text is non-empty
  (`FUN_005f3040`), `FUN_00609650` prefix choice -> row; `+0x160` = row id,
  or the empty id if no row / empty text. hyp: as in the Fleet note, with no
  letter shared the first row is chosen.
- else the base handler `Class_0066e060_vt14`.

## Keys (`FUN_00461590`, slot +0x4c, identical to Fleet `FUN_00463360`)

Tab (9) ignored; Esc (0x1b) closes (slot +0x30); Left/Right (0x25/0x27)
walk to the previous/next visible tab button (`FUN_0060d7e0(id,1)`, which
presses it and fires command 100); hyp: wraps to the first via
`FUN_005f5060` when Right runs past the end (the Left branch uses the same
fallback); Up (0x26) focuses the name box; Down (0x28) focuses the list if
it has rows and reposts the key to it; any other key goes to the parent
(`param_1[8]` slot +0x14 with 0x100), but the galaxy view drops F1..F7
while the finder is open (`+0xb8` bit 0).

## Port notes (hyp:)

- hyp: port `ExplorationStatus` on the system stands in for `+0x88` bit 1
  for the player's side; side bits from the system's controlling faction,
  0xc0 = neutral.
- port: reuse `fleet_finder.rs` structure: `prefix_choice`, row list,
  20 px rows, grey/white colours, modal input, centred window.
- Display/Enter/double-click open the sector window of the system's sector
  (reuse if open) and close; no Fleet window.
- Tab heading strings and title from TEXTSTRA 6245..6249, 6256, 6257.

## Supporting functions

`FUN_0042a000`, `FUN_0042a0c0`, `FUN_00422ce0`, `FUN_00427270`,
`FUN_00460090`, `FUN_00460180`, `FUN_004601a0`, `FUN_00460270`,
`FUN_00460d10`, `FUN_00460fe0`, `FUN_00461000`, `FUN_004611d0`,
`FUN_00461590`, `FUN_004f31b0`, `FUN_004f6010`, `FUN_004f3220`,
`FUN_004f3970`, `FUN_0053ef50`, `FUN_0053f030`, `FUN_0053f090`,
`FUN_004025b0`, `FUN_0042d170`, `FUN_004f26d0`, `FUN_00429ce0`,
`FUN_00604f90`, `FUN_00605070`, `FUN_00609650`.

## The port (2026-10-30)

`crates/rebellion-render/src/system_finder.rs`: `SystemFinderState`,
`SystemTab` (the five filters above, from the port's ownership and
`ExplorationStatus` for the viewing side), the sorted rows, `prefix_choice`
(`FUN_00609650`) and `draw_system_finder`. `main.rs` opens it from F2 and
from the cockpit's 0x12d control, and while it shows the Finders' modal
gate (`finder_modal`) holds the map, the GID control and the cockpit.
Display, Enter and a double click open the system's sector window
(`sector_window_state.open_for_system`) and close the Finder.

- port: a tab change keeps the old choice when the new list chooses
  nothing, as `+0x160` is left; an empty name clears it, as `0x408` does.
- port: Escape is read before the name box takes it, so it closes the
  Finder with the box focused, as `FUN_00461590` does.
- port: the name box's caret is clipped to the box, as a Windows edit
  control's is.
- The tab art is 49 by 41, and the Neutral and Unexplored tabs
  (10508/10509, 10511/10512) key out their blue first pixel like the
  others: A0 `01-f2.png` shows the panel's white at (192, 118) and
  (244, 118).
- port: the window shows at once, without egui's fade-in.
