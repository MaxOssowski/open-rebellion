# The Personnel Finder's SpecForces view against A0 (2026-11-02)

Surface ledger OBJ-03 C018, "SpecForces Finder type-by-system grid".

## A0

The A0 run used the Windows 10 VM and the Alliance's "DS_flip_allian" save, days 165..170. The captures are in `.artifacts/a0-vm/specforces/`, with hashes in `a0-sha256.txt`.

- **`01-alliance-grid-day165`.** F5 opens the Personnel Finder at (62, 50). Its second view button opens the SpecForces view:
  - one row per system: Drall, Mon Calamari, Tralus, Yavin;
  - four type columns under icons;
  - a count in a red box, or "-".

  The boxes of units en route or on a mission (Mon Calamari, Tralus, Yavin) are dark red. Drall's is bright.
- **`03-mon-calamari-personnel`.** Display on Mon Calamari opens its Defenses window. The Longprobe Y-wing Recon Team is listed last. Its menu greys Mission because the team is en route.
- **`02-recon-mission-praesitlyn-day170`.** Display on Tralus opens the window of the team's running mission instead: "Mission at Praesitlyn", Reconnaissance.

## The port's view

`encyclopedia-placement.mjs --window=finders` (Alliance) now presses the SpecForces view button: the finder's (423, 147), 0x36 lower.

- **Header strip.** It compares the strip of the four type icons, the window's (256..370, 114..142), with A0 `01`: 0 of 3,192 pixels differ.
- **The rest of the view.** It matches A0 by eye in `personnel-finder-specforces.png`: the row and column places, the boxes, the dashes and the grey system names. The port's world has other systems (Ghorman, Ithor).
- **Not compared: dark red boxes.** No unit in the port's run was moving or on a mission, so the dark red state of en-route and on-mission units was not compared.

Run `encyclopedia-placement-2026-11-02T12-38-36-124Z` (Arch Chromium, release WASM `e1ad53f8…`): pass.
