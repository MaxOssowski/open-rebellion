# The Fleet Finder

Recovered 2026-10-05 from REBEXE.EXE with Homebrew Ghidra 12.1.3 (read-only,
no analysis), with STRATEGY.DLL's bitmaps, TEXTSTRA and the manual. Item 4
of the fleet-actions plan. Every function named here has a `FUN_<address>.c`
note in this directory.

The manual (pp. 124-125, Figs. 3.68-3.70) gives the player's view:

> Single-click on the **Fleet Finder** control to bring up the Fleet Finder
> dialog box (Fig. 3.70). Enter the name of the fleet to locate it, or
> scroll down the list and select a fleet. Tabs let you search through a
> list of all fleets, Alliance fleets, or Imperial fleets. To open up the
> Fleet window for the selected fleet, click on the **Display** button, or
> double-click on the fleet. Use this same window to search for a specific
> ship. Click on the **Ship Finder** button to bring up the Ship Finder
> control (Fig. 3.72), which works the same as the Fleet Finder control.
> Click on the **Fleet Finder** button to switch back. Click on the
> **Close** button if you don't want to bring up a window for a specific
> ship or fleet.

Fig. 3.70's callouts: "Open Fleet window and Sector window for selected
fleet", "Close Fleet Finder window", "Switch to Ship Finder", "Switch back
to Fleet Finder from Ship Finder".

## Entry

- **The cockpit control.** `FUN_00427270` builds the cockpit's buttons
  (`FUN_00602150`). The Fleet Finder posts `WM_COMMAND 0x12e`: the
  Alliance's at `(157,407)`, 27 by 15, art 10004/10003; the Empire's at
  `(199,434)`, 37 by 24, art 10018/10017. The port already draws it
  (`CockpitButton::FleetFinder`, `docs/qa/2026-09-10-interface-parity-audit/
  evidence/2026-09-11-strategic-command-controls.md`).
- **The galaxy view** (`FUN_00422ce0`): `WM_COMMAND` `0x12d`, `0x12e`,
  `0x12f`, `0x130` open the System, Fleet, Personnel and Troop finders, each
  refused while `FUN_004fcee0()` (`FUN_0051ce00()->+0xc`) is below 2. The
  keys F2..F5 (`WM_KEYDOWN` `0x71..0x74`) reach the same cases: F3 the Fleet
  Finder. Keys `0x70..0x76` are ignored while one of the view's windows has
  `+0xb8` bit 0, as an open finder does, or while Ctrl is down.
- **The opener** `FUN_0042a0c0`: unless a window of type `0x15` is already
  open (`FUN_00604500(+0x6c, 0x15)`), it builds `FUN_00461750(.., 0, 0,
  0x1d6, 0x14a, view, 0x15)`, a 470 by 330 window, shows it and adds it to
  the view's list. The other finders are `FUN_0042a000` (System),
  `FUN_0042a180` (Personnel) and `FUN_0042a4d0` (Troop).

## The window (type `0x15`, vtable `0x0065a090`)

`FUN_00461750` sets `+0xb8` bit 0, `+0x158` the galaxy view, `+0x168` a
list sorted by name (`FUN_0060a790(.., 2)`; `FUN_0060a890` mode 2 compares
names with `FUN_00626ad0`, `_stricmp`), and `+0x16c` the chosen object's id.
The create slot (`+0x38`, `FUN_00461960`) bounds it to the galaxy view's
rect (`FUN_00606980`; hyp: centered there, as the move confirmation) and
builds, by the view's side (`+0x9c`, 1 Alliance, 2 Empire):

| Part | Alliance | Empire |
|---|---|---|
| Background, Fleet Finder | 10335, 10526 at `(12,13)`, 10586 at `(412,0)` | 10336, 10527, 10590 |
| Background, Ship Finder | 10335, 10524 at `(12,13)`, 10586 at `(412,0)` | 10336, 10525, 10590 |
| Close (`200`, TEXTSTRA `0x1954` "Close") | `(423,25)`, 10514/10515 | `(426,21)`, 10516/10517 |
| Display (`0xc9`, `0x1953` "Display") | `(423,93)`, 10518/10519 | `(426,89)`, 10520/10521 |
| Mode group (`0xfa`) | `(423,147)`, 32 by 85 | `(426,143)`, 44 by 136 |
| Ship Finder (mode 2, `0x1885`) | `(0,0)`, 32 by 31, 10530/10531 | 44 by 41, 10534/10535 |
| Fleet Finder (mode 1, `0x1880`) | `(0,54)`, 32 by 31, 10528/10529 | 44 by 41, 10532/10533 |

Both buttons play sound `0x260` (`FUN_00602840`). Every side shares:

- the title, TEXTSTRA `0x1880` "Fleet Finder" (`0x1885` "Ship Finder" in
  mode 2), at `(36,14)`, 350 wide, font 5, drawn into the background;
- the name label `0x1881` "Fleet Name" (`0x1886` "Ship Name") at `(36,48)`,
  font 4, drawn into the background;
- the side tabs (`100`, `FUN_0060d590`) at `(36,78)`, 153 by 41: All
  `(0,0)` 49 by 41, 10500/10501, `0x1882` "All Fleets"; Alliance `(52,0)`,
  10502/10503, `0x1883`; Imperial `(104,0)`, 10505/10506, `0x1884`. In
  mode 2 the labels become `0x1887..0x1889` "All/Alliance/Imperial Ships";
- the name box (`0xcd`, `FUN_00604cf0`) at `(143,45)`, 250 by 16, white
  text;
- the tab's label (`+0x15c`) at `(40,119)`, 283 by 16, font 5: the selected
  tab's text, such as "All Fleets";
- the list (`0xcc`, `FUN_00607ea0`) at `(36,138)`, 350 by 165, rows 350 by
  20, scroll bar art 10653 (`0x299d`).

It opens in mode 1 with All selected (`FUN_00462be0(.., 1)`) and the focus
in the name box. Slot `+0x30` closes it.

## The list (`FUN_00462be0`)

When the tab changes, the list is rebuilt from the view side's objects
(`FUN_0053ef50(range, side, 1)` → `FUN_0053f030`, `FUN_0053f090`: each
object's view for that side, `FUN_005844e0`):

- mode 1 walks fleets (`0x08..0x10`, `FUN_004f3630`), mode 2 capital ships
  (`0x14..0x1c`, `FUN_004f2db0`);
- an object needs `+0x50` bit 6 (existing; spare fleets have it clear,
  `fleet-join-split.md`), and its container (the ship's fleet's, for a
  ship) must not be of family `0xf2`;
- tab 1 lists every such object, tab 2 those with side bits (`+0x24` bits
  6-7) `0x40` (Alliance), tab 3 `0x80` (Empire);
- each row is the object's id and name (`FUN_004421d0`, `FUN_004f6270`:
  its own name, else its record's), inserted in name order
  (`FUN_005f59f0`).

The tab's label is set to the tab's text and the chosen id (`+0x16c`) is
cleared.

## Choosing and opening

The command slot (`FUN_00462770`):

- `100`, a tab: rebuild the list for it.
- `0xcc`, the list: any notification keeps the selected row's id in
  `+0x16c`; `0x29b` (a click) also copies its name into the name box, and
  `0x309` (a double click) opens it.
- `0xc9`, Display: open `+0x16c`. `200`, Close: close.
- `0xfa`, the mode group: `FUN_004632d0` clears the name box, switches the
  background (`FUN_006075e0`), and rebuilds the list for the tab in use.

The name box (`FUN_00462a50`): `0x408` (its text changed) picks the first
row, in list order, with the longest prefix in common with the text,
ignoring case (`FUN_00609650` → `FUN_005f3430` with case flag 0, which
compares through `FUN_0061a620`), selects it and scrolls to it, and keeps
its id; no row shares a first letter, nothing is chosen. `0x407` (Enter)
opens the chosen row.

Keys (`FUN_00463360`): Escape closes; Left and Right walk the tabs; Up
focuses the name box; Down moves into the list.

Opening is `FUN_00429440(view, id)` then close. For a fleet whose container
is a system, it opens the system's sector window (`FUN_00429ce0`) and from
it the Fleet window for that system (`FUN_0045c8e0`, kind 4, as the fleet
icon does, `fleet-window.md`), and selects the fleet there (slot `+0x6c`).
A capital ship opens its fleet's system the same way and selects the ship.
A fleet in a `0xf2` container opens nothing. The sector window opens the
Fleet window at its fleet icon's stored point (`FUN_0045c8e0` →
`FUN_0045aac0(.., item +0x40, +0x44)`).

## Port notes

- hyp: the side's objects are its own and the other side's it has seen. The
  port's Fleet window shows the other side's fleets where the player can see
  a system's contents (`opposing_contents_visible`); the Finder lists those.
- hyp: family `0xf2` is a container that is not a system (the galaxy is
  `0xf1`, the sides `0xf3..0xf4`); every port fleet has a system, so none is
  skipped.
- port: a fleet in hyperspace has already changed container to its
  destination in the original (`move-order.md`), so the Finder lists it.
  The port's fleet leaves its system's list while in transit, which is what
  names it ("Fleet N") and what its Fleet window shows, so the port lists
  only fleets in orbit.
- port: fleet names are the port's labels (`fleet_label`, "Fleet N"); a
  ship's name is its class's.
- port: no scroll bar, as the port's other lists; the wheel scrolls it.
- port: the list keeps its own double click, two clicks on one row within
  egui's double-click delay, since egui counts a double click across
  widgets and a third click as a triple.
- port: the Finder opens centered in the galaxy view and draws above its
  other windows while open: it sits on egui's Tooltip order and raises itself
  each frame, since the modeless windows share Foreground and raise the
  focused one each frame (aeafc17f).
- port: the galaxy view's letter keys stand aside while the Finder is open.
  F1..F7 reach the cockpit once the name box loses the focus; the original
  ignores `0x70..0x76` while a finder is open.
- port: the Fleet window opens at the center of the system's fleet icon,
  which is where the icon opens it without a double-click point
  (`fleet-window.md`).
- hyp: the current tab and mode show their second bitmap, as the mission
  dialog's tab strip does; the selected row draws yellow.
- The port's implementation is `crates/rebellion-render/src/fleet_finder.rs`;
  `tools/interface-parity/fleet-finder.mjs` gates it for both sides.

## Still open

- `FUN_004fcee0`'s state value (below 2 refuses every finder).
- `FUN_00606980`'s placement, and the tab group's pressed art.
- Whether the move confirmation (`FUN_0044f060`) and the mission dialog are
  modal. The port's cockpit controls stay live while either is open, so the
  Fleet Finder control or F3 opens the Finder over them, and the Finder, the
  larger of the two and centered in the same place, hides them until it
  closes.
- The System, Personnel and Troop finders (`FUN_0042a000`, `FUN_0042a180`,
  `FUN_0042a4d0`).

## Supporting decompiles

`FUN_00427270`, `FUN_00422ce0`, `FUN_0042a0c0`, `FUN_0042a000`,
`FUN_0042a180`, `FUN_004fcee0`, `FUN_00461750`, `FUN_00461960`,
`FUN_00462a50`, `FUN_00462770`, `FUN_00462a30`, `FUN_00461860`,
`FUN_00463360`, `FUN_004632d0`, `FUN_00462be0`, `FUN_004f3630`,
`FUN_004f2db0`, `FUN_0053ef50`, `FUN_0053f090`, `FUN_004f6010`,
`FUN_004421d0`, `FUN_005f59f0`, `FUN_0060a790`, `FUN_0060a890`,
`FUN_00626ad0`, `FUN_00609650`, `FUN_005f3430`, `FUN_00429440`,
`FUN_0045c8e0`, `FUN_0041c680`.
