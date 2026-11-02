# The reference rail's window-type icons against A0 (2026-11-02)

Surface ledger CMD-04 C015, "reference-rail window-type icon".

## A0

The A0 run used the Windows 10 VM and the Alliance's DS_flip_allian save, day 178. The captures are in `.artifacts/a0-vm/rail/`, with hashes in `a0-sha256.txt`.

Three windows were minimized one after another, after the Sumitra sector window over the rail was closed. Each icon's non-key pixels are found exact at the slot's corner:

| Capture | Window minimized | Icon | At |
|---|---|---|---|
| `03-rail-defenses` | Yavin's Defenses window | 11533 | (544, 61), slot 0 |
| `05-rail-fleet` | Mon Calamari's Fleet window | 11536 | (544, 83), slot 1 |
| `07-rail-mission` | the "Mission at Selonia" window | 11539 | (544, 105), slot 2 |

The places are the port's `rail_slot_rect` for the Alliance: x 544, rows from y 61, 22 apart.

## The port

- **Defenses.** `sector-quadrants.mjs` minimizes a Defenses window and a Missions window and compares their rail icons with the bitmaps, 0 pixels differing (run `sector-quadrants-2026-11-02T13-41-46-112Z-363794`). The Alliance's Defenses slot against A0 `03`, over the icon and the background beside it, differs in 4 of 234 pixels, where the name's first letter starts.
- **Fleet.** `fleet-window.mjs` `chrome` now also checks the minimized Fleet window's icon at its slot: 11536 for the Alliance and 11537 for the Empire, 0 of 39 and 35 non-key pixels differing. Run `fleet-window-2026-11-02T14-37-32-580Z-466528` (81 cases): pass.
- **Missions.** The port's Missions icon, 11539 for the Alliance, is the one A0 shows.
