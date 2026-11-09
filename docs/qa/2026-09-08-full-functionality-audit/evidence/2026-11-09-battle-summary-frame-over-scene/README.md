# The Battle Summary's frame lies over its scene

Date: 2026-11-09. Adds to TAC-07-C001. Follows
`2026-11-09-encyclopedia-assault-frame-over-panes`.

## The defect

The Battle Summary's frame is the Message Index's base, STRATEGY 10335
(Alliance) or 10336 (Empire), keyed on its blue (palette index 252). Its
top corner arcs reach into the aperture at (12, 13). The port painted the
frame first and the page's art (the scene, the forces columns or the
Display page) over it, so the arcs were covered.

The Battle Alert's frames 10710 / 10711 are not changed. They hold much
more art inside the aperture (14500 non-blue pixels), part of it covered
in A0 (TAC-01-C010: 5792 the frame's colour, 8708 not), so they need
their own trace.

## A0

For every frame pixel that is not blue and lies inside the aperture
(x 12..411, y 13..319):

| A0 capture | Frame | Origin | Frame's colour |
| --- | --- | --- | --- |
| `.artifacts/interface-parity/a0/TAC-01-C006/original.png` | 10335 | (62, 49) | 900 of 900 |
| `.artifacts/interface-parity/a0/TAC-01-C013/original.png` | 10335 | (62, 49) | 900 of 900 |
| `.artifacts/a0-vm/empire-game3/battle-result-coruscant.png` | 10336 | (125, 52) | 474 of 487 |

The 13 others in the Empire's capture lie on the window's last row
(y 319, x 219..234); what covers them there was not traced. All the
top corners' pixels are the frame's.

## The change

`frame_over_page` (`tactical_view.rs`) puts the frame after the page's
first bitmap and before its texts, for the Summary, the forces pages and
the Display page. The unit test
`the_summary_frame_lies_over_the_scene_and_under_the_texts` fails when
the frame is put first.

## Browser

`fleet-window.mjs` `squadron-battle` (Empire, Linux) compares the
Summary's top corners (window x 0..29 and 384..411, y 0..21) with A0
`battle-result-coruscant`, beside the scene, the rail and the headline.
The corners differ in 0 pixels.

`run.mjs` `production-battle-results-presentation` (both sides, 640x480)
used to skip the frame's whole pane and expected the scene's own pixels
at its corners. It now skips only the two text boxes and the rail, so
the frame's pixels over the scene are checked against STRATEGY
10335/10336. The Summary, forces and Display pages skip the pixels the
frame covers.

## Batch b151

23 of 23 interface gates on Linux; workspace tests 3495 passed, fixture
tests 139. Tactical matrix `2026-11-09T06-21-29-079Z-1706343`: 146 of
148. The two failures were `production-battle-results-presentation`,
whose old expectation is described above. After that gate change, the
rerun `2026-11-09T06-36-17-211Z-1738635` passed 4 of 4 for that scenario
(both sides, 640x480 and 1280x800). The other 146 cases ran on the same
build and were not rerun.

Mutants on the diff: 3 caught, 3 unviable, 2 missed. The two missed
replace `draw_battle_result_summary` and
`draw_battle_result_force_details` with `()`. Both are macroquad draw
calls that the unit tests cannot run. The browser scenario above fails
when either page is not drawn.
