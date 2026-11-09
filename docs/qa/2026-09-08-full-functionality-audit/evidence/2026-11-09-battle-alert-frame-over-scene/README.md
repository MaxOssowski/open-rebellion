# The Battle Alert's keyed frame lies over its scene

Date: 2026-11-09. Adds to TAC-01-C010 (battle entry from the Battle
Alert). It follows `2026-11-09-battle-summary-frame-over-scene`, which
left the alert's frame order open.

## A0

`.artifacts/interface-parity/a0/TAC-01-C010/original.png` (SHA-256 in
`SHA256SUMS`) shows the Alliance's Battle Alert at (62, 49) on a list
tab, so with scene 10714. The comparison covers every pixel of frame
10710 over the scene's area (from window (12, 13)) that is not the
frame's key colour (pure blue). For each one it checks whether the
capture shows the frame's colour, the scene's colour, or neither:

| Shows | Pixels |
| --- | --- |
| the frame's colour only | 5931 |
| the frame's and the scene's (the same colour) | 8 |
| the scene's colour only | 0 |
| neither (the title, the list and its bar) | 561 |

So the frame is painted over the scene. The capture for the summary tab
(scene 10712) is not among the A0 cells. The order is taken to be the
same for both scenes and both sides, as for the Battle Summary.

## The change

`battle_alert_layers` (`tactical_view.rs`) lists the scene (at (12, 13),
cut to 400 by 310) and then the frame. `draw_original_battle_alert`
paints them in that order. Before this change the frame was painted
first. The unit test `the_alert_frame_lies_over_the_scene` fails when the
frame comes first.

## Browser

`run.mjs`'s Battle Alert probe used to skip the frame's whole aperture.
It now skips only the scene's text boxes, the bottom row and the rail,
so the frame's pixels over the scene are checked against STRATEGY
10710/10711. The scene and list-scene checks skip the pixels the frame
covers.

Only pure blue (index 252) counts as the key in these checks. Frame
10711 has a dark blue, (0, 0, 223), inside the aperture. Both the port
and A0 `.artifacts/a0-vm/battle-alert-empire/00-alert-tralus-day300.png`
show it over the scene, for example at window (12, 62).

## Batch b152

23 of 23 interface gates on Linux; workspace tests 3496 passed, fixture
tests 139. Tactical matrix `2026-11-09T07-30-51-332Z-1830003`: 147 of
148. The failure was the Empire's `production-battle-alert-entry`. The
first version of the gate mask treated that dark blue as key colour.
With the mask limited to pure blue, the rerun
`2026-11-09T07-46-32-533Z-1856085` passed 4 of 4 for that scenario. The
other 147 cases ran on the same build and were not rerun.

`cargo mutants` generated no mutants for the diff. `battle_alert_layers`
returns a fixed array of tuples, which it cannot replace with a default
value. The unit test above covers the order, and it fails when the frame
is listed first.
