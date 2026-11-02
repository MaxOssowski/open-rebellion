# Build Selection's number, Cancel, and a band's selection after its menu (2026-11-02)

Surface ledger:

- OBJ-08 (Build Selection): C009 "quantity" and C012 "cancel".
- OBJ-07 (Manufacturing): C007 "queue populated".

## A0

The captures are in `.artifacts/a0-vm/build-selection2/`, with hashes in `a0-sha256.txt` (see `2026-11-02-production-stop-confirmation`). They come from the Empire's "resource turn 14" save in the Windows 10 VM. Coruscant's window is at (203, 91) and Build Selection at (255, 87).

- **`01`..`04`.** The Imperial Army Regiment at Coruscant, with one training facility.

  | Number | Refined | Maintenance | Completion | Deployment |
  |---|---|---|---|---|
  | 1 | 6 | 3 | 24 Days | 0 Days |
  | 3 | 18 | 9 | 72 Days | 0 Days |
  | 2 | 12 | 6 | 48 Days | 0 Days |

  The costs and the single yard's completion scale with the number. The deployment does not.
- **`05`, `06`.** Cancel closes the window and the band is unchanged. Build reopens the window at 1.
- **`07`.** After Confirm, the band shows:
  - the name and "Training: 1";
  - the mini GOKRES 1154 at the window's (95, 153), with its 96% non-key pixels found. The port puts it at the band's (40, 15), which is the same place.
  - an unselected title strip.
- **`16`..`18`, the band's selection** (strip colour at x 380):

  | Capture | Ships | Troops | Facilities |
  |---|---|---|---|
  | `16` after a left press on troops | (91, 179, 75) | (0, 255, 0) | (91, 179, 75) |
  | `17` with the facilities band's menu open | (91, 179, 75) | (91, 179, 75) | (0, 255, 0) |
  | `18` after Escape | (91, 179, 75) | (91, 179, 75) | (91, 179, 75) |

  A left press keeps its band selected. A band's menu selects its band only while it is open.

## The port

- **Deselecting.** `SystemWindowState::deselect_band` runs in `main.rs` when an object menu opened for a `MenuObject::Producer` closes, whichever row was chosen or none. Before this change, the band kept its lit strip after its menu.
  - hyp: the right press selects only for the menu. The end of `FUN_004ac5c0` is untraced.
- **Fixture report.** The production report now carries Build Selection's number and its four values as drawn: `build_selection_quantity` and `build_selection_values`.
- **`build-selection.mjs`, on both sides:**
  - presses the up arrow twice: number 3, the costs and completion tripled, the deployment unchanged;
  - presses the down arrow once: number 2;
  - presses Cancel: the window closes and the band stays empty;
  - reopens with Build: number 1.
  - For the Empire, it also compares 21 by 10 text-free pixels of the confirmed band's title strip with A0 `07`. They must be equal.
- The fixture builds another class, the Stormtrooper Regiment, so the values are checked for their scaling, not against A0's numbers.

## Tests

- `a_band_menu_closing_leaves_no_band_selected_in_that_window_only` (`system_window.rs`).
- `the_menu_reports_each_rows_label_and_state_in_order` now also checks `ObjectMenuState::selection` (`object_menu.rs`).

## The idle overview (OBJ-07 C006, C008)

A0 `11-stopped` shows Coruscant with every band idle. `build-selection.mjs` compares the Empire's idle overview with it (`idleBandsAgainstA0`).

Each band's status line (TEXTSTRA 6198..6200 at the band's (5, 16)) and the troops' and facilities' "1:1" counts (`FUN_00457690`) are compared by their ink boxes relative to the window. Run `build-selection-2026-11-02T08-35-16-181Z-3818211` (Linux Chromium 155, test WASM `354677a3…`):

| Text | A0 | Port |
|---|---|---|
| "No Ships are being built" | 60, 73..204, 84 | 61, 72..204, 84 |
| "No Troops in training" | 60, 157..157, 165 | 60, 157..158, 165 |
| "No Facilities are being built" | 60, 238..185, 246 | 61, 238..186, 246 |
| troops "1:1" | 21, 203..34, 212 | 21, 204..37, 212 |
| facilities "1:1" | 21, 283..34, 292 | 21, 284..37, 292 |

- The counts' 3 extra pixels are Liberation Sans's footed "1" against GDI Arial's plain one. This is the accepted text deviation.
- The band titles are black in both: A0 draws them crisp, the port antialiased.
- A0's Coruscant has no shipyard ("0:0"), so the ships' count is not compared.

## How a band's menu closes (corrected the same day)

`20`..`24` refine the rule above. They sample the strip colour at x 380:

| Capture | Ships | Troops | Facilities |
|---|---|---|---|
| `21`, after the ships band's Destination released on Chandrila | lit | | |
| `22`, the facilities band's menu | | | lit |
| `23`, that menu dismissed by a click beside it | | | lit |

Only the ships band changes its destination ("Destination: Chandrila"). The others keep Coruscant, which shows Destination stored per producer.

So the selection ends on:

- Escape (`18`);
- Build and Stop, whose windows open over it (`05`, `07`, `11`, `15`).

It stays after Destination and after a click beside the menu.

- `system_window::band_menu_deselects(command, escaped)` holds this rule. `main.rs` applies it when a band's menu closes.
- hyp: the windows' activation clears the selection. The other rows are not captured and keep it.
- Test: `escape_build_and_stop_end_a_bands_selection_but_destination_and_a_click_beside_keep_it`.
