# Personnel Finder: the two views, and the full row text (P08)

Date: 2026-10-22. The reading is in `ghidra/notes/personnel-finder.md`.

## The views were swapped

`FUN_004637a0` builds the view group (`0x6e`):
- value 3, labelled TEXTSTRA `0x1899` "Characters", with a face (10542),
  at the group's top;
- value 2, labelled `0x1898` "SpecForces", with two soldiers (10546),
  `0x36` lower.

It opens on the player's side and view 3 (`FUN_0060d7e0(side, 1)`,
`FUN_0060d7e0(3, 1)`). `FUN_00464e10` lists characters for view 3 and
special forces for view 2:
- it shows panel 2 or 3 (10538 or 10539, the bitmap with the four
  SpecForces column icons) for view 2, and 4 or 5 (10536 or 10537) for
  view 3; the panels are laid at `0x463b78..0x463bdf`;
- the label's y is `0x77` for view 2, else `0x73`.

The earlier notes swapped the two labels. The port followed them: it
swapped the panels, the button positions and the label y, and it opened
on SpecForces. Two tests had locked that in with the wrong citation.
Their expectations now follow the evidence above.

## The row text (`FUN_00465bb0`)

- A destroyed character reads "Killed" (`0x8798`), or "Retired"
  (`0x8799`) for destruction reasons `0x14..0x16`. The port retires no
  one.
- A character in an abode (container family `0x98..0x9f`: the jails,
  Dagobah, the Hermitage, Jabba's Palace) shows the abode's name alone.
  The port gave such a character "Enroute".
- The state word now includes Injured (`0x8747`, `+0x94`), after
  Captured and On Mission.
- The command word follows the state (`+0x96`: Commander `0x8802`,
  Admiral `0x8801`, General `0x8800`).

## Who is listed

Both sides' created characters, each under its own side tab, from the
player's side's view of each one. The port has no per-side views, so it
shows an enemy character's true place.

## Gates

- Tests: 2 new (row text; panels and buttons). 2 corrected (the opening
  view; the label y). The row-text test fails on the old code. Workspace
  2669 passed, 0 failed.
- Mutants: 20 of 20 caught, 1 unviable.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build):
  - GID catalog: 152 of 152;
  - `audit-batch` (F5 opens the Personnel Finder; its rail matches the
    original): pass.
  - The screenshots show the Finder opening on the Characters view, with
    the Characters panel.
- Replay equivalence passed.
- Release WASM `8b62b0e8e68405b452d9463a646b979b362284aa44bbb2429e2d1f27c6d5634d`.
