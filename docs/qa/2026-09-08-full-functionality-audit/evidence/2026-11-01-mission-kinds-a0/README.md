# Mission kinds in Create Mission against A0 (2026-11-01)

A0: the Windows 10 VM, the Empire Death Star save on day 599-600,
Coruscant. Captures `.artifacts/a0-vm/kinds/`, hashes in `a0-sha256.txt`;
the Alliance's from `menus2/11-13` (Vjun).

## What A0 lists

| Team, release | Kinds, in the box's order |
|---|---|
| Ozzel, his own cell at Coruscant (menus3/53) | Diplomacy first |
| Darth Vader, his own cell at Coruscant (`01`, `03-scroll-*`) | Recruitment, Espionage |
| Darth Vader on Chandrila, held by the Alliance (`20`, `23-hold-*`) | Incite Uprising, Espionage |
| Mon Mothma on Vjun, the Alliance's own (menus2/11-13) | Recruitment among them |

The drop-down scrolls by pixels while its down arrow is held; the
wheel does not scroll it.

## The kind's item against the port

Each kind's 130 by 65 GOKRES icon (TEXTSTRA id & 0xfff, + 0x1000 for the
Empire, `kind_icon`) was found in the captures and compared row for row
with the resource:

| Kind | GOKRES | Where | Rows 0..63 differing |
|---|---|---|---|
| Diplomacy (Empire) | 7184 | box, menus3/53 | 0 of 8,320 |
| Diplomacy (Empire) | 7184 | Mission Status, `12` | 0 of 8,320 |
| Recruitment (Empire) | 7190 | box, `01` | 32 (the pointer over it) |
| Espionage (Empire) | 7187 | drop-down, `03-scroll-2` | 0 of 8,320 |
| Incite Uprising (Empire) | 7232 | box, `20` | 0 of 8,320 |
| Recruitment (Alliance) | 3094 | box, menus2/12 | the tooltip over it; the rest matches |

The icons' 65th row is the blue key in every resource, and A0 shows the
background there in the box, the drop-down and the Status window: the
item (`FUN_0046a9c0` -> `FUN_005fd0f0`) and the Status window blit them
keyed. The port drew that row blue; GOKRES `0xc00..0xd02` and
`0x1c00..0x1d02` are now blue-keyed (`uses_blue_screen_transparency`).
The name sits white along the box's top in font 4 (`2026-11-01-menus2-a0`).

The Missions window's row shows the kind's mini (`0x5c00` range), not
this icon (`10`).

## Open

The other kinds (Abduction, Assassination, Death Star Sabotage, Subdue
Uprising, Jedi Training, Reconnaissance, Rescue, Sabotage, the three
research kinds) still have no A0 capture: they need a character target,
a captive, an uprising, the Death Star as target, or a research team.
