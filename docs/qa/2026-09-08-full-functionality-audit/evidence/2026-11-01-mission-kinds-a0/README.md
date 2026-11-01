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

## The drop-down

A0 `23-hold-*`, `31-needa-*`, `32-veers-*`, `32-emperor-*` (hashes in
`a0-sha256-lists.txt`): the list holds every kind, one 113-pixel item
under the other; the shown kind's name is white and the others' gray
(0x808080, the item's `+0x60`); its RCDATA 0x299a bar shows at the
right and its held down arrow scrolls the list by pixels; the wheel does
nothing; the window's background is STRATEGY 10598 tiled from its
top-left inside a white one-pixel frame at (0, 0)..(198, 115). The port
showed one item at a time, white, on black, scrolled by the wheel.

Now `draw_mission_dialog` lays every kind 113 pixels apart from the
list's first visible pixel, names the shown one white and the others
gray, draws the bar with the Status window's 0x299a art (step 10; hyp:
`FUN_0060a490`'s step not read), opens on the shown kind, chooses the
item under a click (the bar only scrolls), and paints the tile and the
frame. The bars' arrows (10658..10664, 10685..10688, 10750..10753) are
strobe buttons and now keyed on their blue corner, as A0 shows the
track there. Against `31-needa-00` the frame and the bar match but the
thumb's length (the fixture lists more kinds than Needa) and two key
pixels, now keyed.

Tests: `the_kind_list_scrolls_by_pixels_on_its_bar_and_a_click_chooses_an_item`,
`the_kind_list_opens_on_the_shown_kind_and_lays_its_items_113_apart`,
`a_click_chooses_the_item_under_it_at_any_scale`,
`the_kind_list_names_the_shown_kind_white_and_the_others_gray`.
`mission-dialog.mjs mission` captures the open list on both sides (run
`mission-dialog-2026-11-01T15-34-51-086Z-1170469`):

```
44700212ab9031f39923f8f7116731279c42fd9c9192eee3284eb8aced9fbb73  alliance-mission/kind-list-screen.png
a93a1a78557bfcd3fbb9a3d62b3f1137ebd5a0be0e630ac04fc1a906d36c204d  empire-mission/kind-list-screen.png
```

## Open

The other kinds (Abduction, Assassination, Death Star Sabotage, Subdue
Uprising, Jedi Training, Reconnaissance, Rescue, Sabotage, the three
research kinds) still have no A0 capture: they need a character target,
a captive, an uprising, the Death Star as target, or a research team.
