# Facility pictures and empty deposits against A0 (2026-11-03)

Surface ledger cells: CMD-09 C001 (mine active), C004 (refinery active) and C009 (shipyard en route). This also corrects OBJ-07 C004 (mines): the empty deposit's offset.

## A0

The run used the Alliance save at day 303 (Windows 10 VM, original `REBEXE.EXE`). It shows Mon Calamari's System window, with its window at (314, 86). The captures are `.artifacts/a0-vm/personnel-states/34-*` (ignored); the hashes are in `a0-sha256.txt`.

Every picture was found in the capture, pixel for pixel over its opaque pixels. The blue matte is keyed.

| Capture | Picture | At (window) | Off |
|---|---|---|---|
| `34-mon-calamari-shipyards-day303` | 9009, the shipyard en route | (9, 79) | 1 of 2345 (a star under the art) |
| `34-mon-calamari-construction-yards` | 9022 | (9, 79) | 0 |
| `34-mon-calamari-refineries` | 9030 ×5 | (9, 79), (78, 79), (147, 79), (9, 119), (78, 119) | 0 |
| `34-mon-calamari-mines` | 9001 ×2 | (9, 79), (78, 79) | 0 |
| `34-mon-calamari-mines` | 9005 ×3, empty deposits | (146, 77), (8, 117), (77, 117) | 0 |

`FUN_00458fe0` picks base + 3 when `+0x50` has bits 4 and 0x10 set, so the shipyard is en route. It did not change while the game ran on (the day stayed at 303).

## The correction

An empty deposit sits at its cell's corner, (0, 0), not at (0, -2).

The 2026-11-01 reading of A0 `manufacturing/05` (`evidence/2026-11-01-manufacturing-pages-a0/`) took the deposits from the title bar at (205, 93) rather than the window at (203, 91). Measured from the window, the deposits there are at (146, 117), (8, 157), (77, 157), (146, 157) and (8, 197), the same corner as in Mon Calamari's capture. The mines in that capture were already measured from the window.

`DEPOSIT_OFFSET` is now (0, 0), and the unit test expects (77, 77).

## The port

The fixture adds a built mine (0x2c000001) and refinery (0x2d000002) to the build-selection system.

`build-selection.mjs` now also opens the Refineries and Mines pages. With every opaque pixel exact on Linux Chromium 155 for both sides, it finds:
- 9030 at (9, 79);
- 9001 at (9, 79);
- the first empty deposit, 9005, at (77, 77).

Run `build-selection-2026-11-03T19-59-23-944Z-3486864`: pass.

```
57b2f204563a9f6075faae4ba9f80976a91cd389fc2a7a28fec11481ec963c0e  alliance/mines-page.png
6edc364bf3101a182755ccedd34731ba08aa5e1a55cb47a6132f5b1e2ccecb82  alliance/refineries-page.png
b3d3dec1f0e1894b7d71bd2d38125c8769648a89983de76a136132f9af3fdeaf  empire/mines-page.png
38b763fb239714a4ef9dc8a7d906e72b0b2dc7cd5eaa30d5ef82e1bf22c71911  empire/refineries-page.png
```

## Open

- The shipyard en route (9009) has A0 evidence but no browser case.
- The pictures for facilities under construction have neither A0 nor browser evidence.
