# A double click on a planet opens nothing

Date: 2026-11-09. Corrects CMD-03-C012 (double-click object) and informs
OBJ-02-C011 (the System window's unknown state).

## A0

Windows 10 VM, original REBEXE, the new Alliance game of
`2026-11-09-build-selection-classes-and-list-a0`, day 287:

- Moddell's sector window, opened from the Planetary System Finder's
  Unexplored Systems tab (`00-moddell-before`);
- a double click on unexplored Adega, by two methods (two quick presses
  after a 0.8 s rest; `g.py dc`): nothing opens (`01`);
- a double click on Flax in the Sumitra window, a system with no icons:
  nothing opens (`02`);
- a double click on Akrit'tar: nothing opens (`03`).

The older captures `system-windows/04-yavin-dbl` and `05-yavin-dbl2`
(2026-10-31, a double click on Yavin's planet) show the same. There,
`07-icon-dbl` opened Yavin's System window only from its system icon.

The same QMP double click opens a Manufacturing window from a facility
icon (Kessel, Yavin, Corfai, Duros in this session). So the input was
delivered, and the planet does not answer it.

All captures are in `.artifacts/a0-vm/`, with their SHA-256 in `SHA256SUMS`.

## The trace

`FUN_004593e0` case `0x203` (`WM_LBUTTONDBLCLK`) hit-tests the overlay
list `+0x174` with `FUN_0045cc10`. When no overlay takes the point, it
returns without acting. Only an overlay's action follows
(`FUN_0045b1b0`, `FUN_0045aac0`). The planets are drawn from the list
`+0x164` and never reach that path.

## The change

The port opened the System window on a double click on the planet. That
was invented, and it is gone. The System window opens from its system
icon, as before.

The palette command `Open System window: <system>` (debug and native
builds) now needs the System icon shown, like the other windows.

Unit tests (rebellion-render):

| Test | Before | Now |
| --- | --- | --- |
| `without_icons_a_double_click_on_the_planet_opens_nothing` | `without_fleets_the_icons_corner_opens_the_system_window`, expected `OpenSystemWindow` | expects no action |
| `a_hidden_icon_refuses_its_window_and_menu` | expected the System window to open without an icon | expects "the icon is hidden" for the System window too |

Both expectations changed on the A0 and trace evidence above. Both tests
fail against the old code, and the other 1403 render tests pass.

`agent_docs/agent-tooling.md` and `ghidra/notes/headquarters-overlay.md`
are updated. The latter closes its open question on a planet's own
double click.

## OBJ-02-C011

An unexplored system shows no icons (A0 Moddell). A double click on its
planet opens nothing, and the System Finder's Display opens its sector
window. No A0 path that opens an unexplored system's System window was
found. The cell stays partial. Whether the state can be reached at all
is open.

## Batch b161

23 of 23 interface gates on Linux, `sector-quadrants` and `fleet-window`
among them; they double-click icons, never planets. Workspace tests 3496
passed, fixture tests 139. The diff's mutants: 2 caught, 1 unviable.
Tactical matrix `2026-11-09T21-58-54-327Z-3100795`: 148 of 148.

## 2026-11-10: OBJ-02-C011 excluded

Under the owner's decision of 2026-11-10 (states absent from the
original are removed), OBJ-02-C011 is excluded. In the original, an
unexplored system's System window cannot be reached:

- the unexplored system shows no quadrant icons (A0 Moddell,
  `FUN_0045c240`);
- a double click on its planet opens nothing (above);
- the System Finder's Display opens its sector window.

The port's only other opener, a battle result's system button, needs a
battle there. A battle needs a fleet there, and a fleet there explores
the system.
