# The producer's Status name, a shipyard en route, and the band states (2026-11-04)

## A0

The hashes are in `a0-sha256.txt`.
- **`pending-facility/34`** (Pzob, Alliance, day 633, the Status window at (108, 79)). The ships band's Status reads "Ship Construction" with these rows:
  - Location: Pzob;
  - Status: Building;
  - Items to Build: 1;
  - Estimated Day of Completion: 644.

  The picture is GOKRES 262, 0 of 10728 pixels off. The name is "Shipyard".
- **`personnel-states/34`** (Mon Calamari, day 303, the window at (314, 86)). The Shipyards page shows a shipyard en route, 9009, at (9, 79). Keyed on the bitmap's first byte, 0 of 2344 pixels are off.
- **`pending-facility/33`.** The ships band's menu, while it builds, greys Build, Stop, Destination, Rename and Reserved. The port leaves them white. It is not known why: the facilities band's menu at Pzob on day 591 was white (`pending-facility/35`), but on day 633 it was grey too (`pending-facility/36`), with only one band selected. On day 633 the clock did not advance, though the speed menu was set to Medium and no dialog was showing. So the grey rows may follow a stopped clock. This stays open.

## The reading

`FUN_004f62d0` gives an object's own name (`+0x34`), or else its class's. The manager classes' names are TEXTSTRA 11524 "Construction", 11525 "Shipyard" and 11526 "Training". They come in the order of the pictures 263, 262 and 264 (`FUN_00443130`). The port named the manager after its system (a hyp).

## Now

- **`status_rows::manager_name`.** The producer's Status names the manager class.
- **The fixture.**
  - `place_deliveries` sends an Orbital Shipyard from the secondary to the primary in the Build Selection scenario. The shipyard departs on day 100000, so it stays en route.
  - The status report carries the name and covers producers.

## Tests

- **Unit tests.**
  - `a_producers_status_names_its_manager_class`;
  - `the_build_selection_scenario_sends_a_shipyard_to_the_primary`, which checks the page's last cell, 9009.
- **`build-selection.mjs`** (Linux, both sides).
  - The Shipyards page's third cell is 9009, every opaque pixel exact. For the Alliance, A0 `personnel-states/34` matches the same bitmap.
  - The ships band's Status (fixture 95) reads "Ship Construction" and "Shipyard", with Status "Building" and the four row labels. For the Alliance:
    - the picture matches GOKRES 262 exactly in the port and in A0 `pending-facility/34`;
    - the name's ink box is A0's, (279, 140)..(333, 152).

## The ledger

- **CMD-09-C009** (shipyard en route) passes.
- **OBJ-09-C009** (cancelled) passes on the evidence already gated:
  - A0 `build-selection2/09..15` shows the Stop question and the band emptied by it (`11-stopped`);
  - `build-selection.mjs` Stops the band and compares its idle lines with `11-stopped`.
- **OBJ-09-C010** (destination changed) passes on A0 `build-selection2/21` and `build-selection.mjs`'s destination case, which the band names (OBJ-07-C015).
