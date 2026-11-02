# The Defenses window's Shields and Fighters pages against A0 (2026-11-02)

Surface ledger OBJ-02: C007 "fighter tab" and C008 "shield tab".

## A0

The A0 run used the Windows 10 VM, the Empire's save and Coruscant's Defenses window at (196, 91). The captures are in `.artifacts/a0-vm/status/` (2026-10-30):

- **`17-def-tab`.** The Planetary Shields page, empty, its label "Planetary Shields" at the window's (67..172, 50..63).
- **`19-tab3`.** The Fighter Squadrons page: nine TIE Fighters in rows of three, with a scroll bar. Its label is at (65..168, 50..66).

## The port

The `defenses` case of `sector-quadrants.mjs` now clicks the Shields and Squadrons tabs after the tab tips. For each page it measures the label's ink box relative to the window:

| Page | A0 | Port |
|---|---|---|
| Shields | 67..172, 50..63 | 67..166, 51..63 |
| Squadrons | 65..168, 50..66 | 65..168, 51..63 |

- **Tolerance.** Left and top within 2 pixels, width within 15%. The Shields label is 6 pixels narrower: GDI's hinting, the accepted text deviation. A0's 66 is the descender of q and g.
- **Earlier checks.** The tabs' art (normal, selected, empty) was already compared by the same gate.
- **Run.** `sector-quadrants-2026-11-02T18-32-44-125Z-784713` (Arch Chromium, both sides): pass.
