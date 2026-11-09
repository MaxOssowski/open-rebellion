# Three tactical A0 captures compared in the browser gate

Date: 2026-11-09. Covers TAC-01-C006, TAC-06-C004, TAC-01-C007 and
TAC-04-C011. The A0 captures existed since 2026-10-24, but the gate had
never compared them.

## A0

| Cell | Capture | SHA-256 |
| --- | --- | --- |
| TAC-01-C006, TAC-06-C004 | `.artifacts/interface-parity/a0/TAC-01-C006/original.png` | `885a80354c0df489b793eb86124a0969673253b930f8fbfb9cf967180efdd8b2` |
| TAC-01-C007 | `.artifacts/interface-parity/a0/TAC-01-C007/original.png` | `988651267e31249f34c40fb75d781b25211d5356e4e62504db50d5d2c3435749` |
| TAC-04-C011 | `.artifacts/interface-parity/a0/TAC-04-C011/original.png` | `273f304034bb605941e5f6df474962447b658a10693d0d5175f22582c42a6288` |

All three come from the Windows 10 VM on 2026-10-24
(`.artifacts/interface-parity/a0/manifest.json` has the input traces).

## Get Immediate Result (TAC-01-C006, TAC-06-C004)

A0: the Alliance's Battle at Coruscant, ended by Game Options, then Get
Immediate Result. The Battle Summary opens at (62, 49).

The gate's `production-battle-options-presentation` (Alliance, 640x480)
presses Get Immediate Result, then compares the Summary's rail and its
five buttons (x 476..525, y 60..344) with A0.

### The defect it found

On the b146 build, 63 pixels differed: the corners of the fifth button,
the Display tab (STRATEGY 10791, `FUN_0046f360`). Like every other button
on the rail, its stored first pixel is pure blue (index 252). A0 shows the
frame's grey there. The port keyed its pressed art 10792, but not its
resting art 10791, so the blue corners showed.

`bmp_cache.rs` now keys 10791 too. The unit test
`the_battle_windows_art_keys_out_blue` lists it, and it fails without the
change. The same rail in A0 TAC-01-C013 also shows grey at those corners.
After the fix the gate expects 0 differing pixels.

The texts were already compared word for word by the unit test
`a_failed_alliance_attack_reads_as_a0`. The fixture's battle (Akrit'tar)
is not A0's (Coruscant), so the scene picture and the texts in the browser
are not compared with this capture. The window frame was already checked
against STRATEGY 10335 at A0's origin.

## Empty space (TAC-01-C007)

A0: the Empire's Battle at Yavin, day 234, with Show Planet off. The first
paused frame shows only the starfield and the holocube's blue edges.

The gate's `production-empty-space-presentation` (Empire, 640x480) now
compares the battle view (x 17..456, y 30..465):

- A0's 1334 blue edge pixels and the port's 1381 lie within 2 pixels of
  each other: 99.6% of A0's and 100% of the port's (measured on the
  2026-11-07 run `2026-11-07T12-40-28-275Z-3690805`).
- Lit pixels that are not edges: A0 732 (stars, one hull, the status
  line), the port 1046. A planet backdrop would light thousands. The gate
  allows under 2000.

The opening camera is `FUN_005d9640` and the cube `FUN_005d4d10`, as in
TAC-01-C002 (corner (157, 290) vs the port's (157, 288)).

## Centre on the subject (TAC-04-C011)

A0: the Alliance's Battle at Mon Calamari. The Victory Destroyer is
selected, then the centre of the camera sphere is pressed (1058/1059).
The subject's green box is centred at (237, 247), the middle of the battle
view (17..456 by 30..465).

The gate's `camera-journey` (640x480) presses the same control. The
selected hull's marker is then centred at (237.5, 247). The gate allows 1
pixel.

## Not compared

- TAC-04-C014 (F1 chase camera): A0 follows Task Force 1 from behind in a
  different battle; the framing depends on the fleet's layout.
- TAC-01-C013 (return to strategy): see `2026-11-09-battle-window-underlay-a0`
  for the Summary over the live command center.

## Batch b147

23 of 23 interface gates on Linux (Arch Chromium); workspace tests 3492
passed; the diff's 4 mutants caught. Tactical matrix
`2026-11-09T01-24-22-762Z-468210`: 147 of 148. The failure was this new
centre check on the Empire's side: it looked only for a green marker,
and the Empire's selected hull is marked red. The check now takes either
colour in a narrower box (x 222..251), and the rerun
`2026-11-09T01-38-07-552Z-498711` passes for both sides, each at
(237.5, 247).

Measured in the batch: the simulated Summary's rail 0 pixels from A0;
empty space 1329 of A0's 1334 edge pixels and all 1381 of the port's
matched, 1046 lit non-edge pixels.
