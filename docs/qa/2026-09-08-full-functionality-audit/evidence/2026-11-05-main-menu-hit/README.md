# The main menu's controls hit on their bitmaps' opaque pixels, as A0 (2026-11-05)

This covers PRE-02-C002 ("14 control hotspots"), in part.

## A0

The VM's main menu. Before each click the pointer is walked home. The selected difficulty is read from the red bracket's pixels, and the lever from its region against the previous frame. The captures are in `.artifacts/a0-vm/main-menu-hit/`.

| Probe | Point | In the rectangle | On the bitmap | A0 |
|---|---|---|---|---|
| Intermediate, bottom-left corner (`bl`) | (126, 73) | yes | key | Easy stays |
| Intermediate, top-right corner (`tr`) | (169, 43) | yes | key | Easy stays |
| Intermediate, control (`ctl`) | (144, 73) | yes | opaque | Intermediate chosen |
| Intermediate, row 18 (`x128`, `x132`, …) | x 128 / 132 | yes | key / opaque | none / chosen |
| Easy's keyed corner (`easyDeep`) | (107, 44) | yes | key | Intermediate stays |
| Lever's keyed area (`leverDeep`) | (258, 285) | yes | key | lever unchanged |
| Lever, control (`leverSolid`) | (275, 308) | yes | opaque | lever moves |
| Empire's keyed corner (`empireDeep`) | (156, 311) | yes | key | no game |
| Alliance's keyed corner (`allianceDeep`) | (492, 310) | yes | key | no game |
| Quit's keyed corner (`quitDeep`) | (539, 396) | yes | key | no question |

The points are chosen 7 by 7 deep in each bitmap's key or opaque area, so the QMP pointer's few-pixel uncertainty does not matter. The single-column probes on row 18 (x 129: none; x 132: chosen) are within that uncertainty and are not used.

## Before

`main_menu::hit_test` took a control anywhere in its `FUN_00405560` rectangle.

## Now

- **`hit_test_exact`.** The menu's hover, and so its clicks and tips, go through it. A press takes an animated or strobe control (`FUN_00603870`, `FUN_00602d30`) only on its current bitmap's pixels that are not the first pixel's colour (the `BmpCache` hit mask). Without the bitmap it falls back to the rectangle.
- **The galaxy-size screens.** They stay rectangles, because they are pointer regions of the window (`FUN_00405050`).
- **Unit test.** `a_press_takes_a_control_only_on_its_bitmaps_opaque_pixels` does not compile without the change, since `hit_test_exact` is new. Mutants: 6 caught, 1 unviable.
- **`main-menu-states.mjs`** (Linux, against A0 `main-menu-states/`):
  - **The corners.** Two new steps press the Intermediate monitor's keyed corners. Easy stays chosen, and its region matches A0 frame 06, 0 differing. Under the old rectangle test, Intermediate would have been chosen.
  - **The Alliance corner.** A click on (492, 310) starts no game.
- **The b80 batch.** It ran all 21 Linux gates, and every one passed (in-diff mutants: 9 caught, 2 unviable). `main-menu-rest.mjs` also passes.

## Still open

The galaxy-size screens, Game Type, Load/Options, Credits and Multiplayer have no deep key area to probe, so A0 settles their rectangles only through their bitmaps' shapes.
