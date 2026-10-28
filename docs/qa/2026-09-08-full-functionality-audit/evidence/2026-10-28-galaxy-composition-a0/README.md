# Galaxy view composition against A0 (P06, CMD-02)

Date: 2026-10-28. Docs only. This settles P06's open "synthetic galaxy
composition".

## Method

- The A0 galaxy view: Windows 10 VM, original `REBEXE.EXE`, the "test"
  save, Alliance, Popular Support, day 7
  (`.artifacts/a0-vm/2026-10-28-nav/galaxy-a.png`, SHA-256
  `6e5ccf7739fece0cb058d64de7f1930998fb5cf4208c31e363da498a875584d3`).
- The port: the GID gate's Alliance Popular Support capture at 640 by 480
  (`.artifacts/interface-parity/2026-10-28T18-31-34-150Z-3768598/gid/alliance/popular-support/640x480/actual.png`,
  SHA-256 `981f080c23d4a9995f67134bf9955a77e475f819c740265c0c8d4c8cd8d3b1f8`).
- The two compared pixel for pixel, as captured.

## Result

| Region (canvas) | Identical pixels |
|---|---|
| whole canvas | 301,917 of 307,200 (98.28%) |
| galaxy aperture (55..540, 40..390) | 166,724 of 169,750 (98.22%) |
| top bar | 25,269 of 25,600 (98.71%) |
| cockpit bottom | 54,251 of 54,400 (99.73%) |
| cockpit left | 25,323 of 26,400 (95.92%) |

Every differing pixel belongs to state that differs between the two games
or to animation:
- the GID markers: the systems' sides and support differ;
- the caption's antialiasing;
- the day number and the resource readouts;
- the mouse cursor in A0;
- C-3PO's idle frame;
- the advisor button (lit in A0, the advisor's state);
- the animated globe icon at the bottom left.

The starfield, the shell, the aperture and the controls are pixel-identical.

## Consequence

The galaxy view is no longer synthetic: P06's known visual failure
"synthetic galaxy composition" is removed. The advisor button's lit state
belongs to the droid advisors (CMD-07, tracker item "Droidy-doradcy").
