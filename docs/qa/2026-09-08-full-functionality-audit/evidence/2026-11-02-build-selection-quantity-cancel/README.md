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
