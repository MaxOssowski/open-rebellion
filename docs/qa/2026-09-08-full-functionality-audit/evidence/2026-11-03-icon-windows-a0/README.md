# The windows a sector icon opens, against A0 (2026-11-03)

This covers surface ledger cell CMD-03 C020, "quadrant icon destination window and selected object".

A0 ran the Alliance save at days 303–331 (Windows 10 VM). The captures are `.artifacts/a0-vm/personnel-states/` (ignored); the hashes are in `a0-sha256.txt`.

Each icon is double-clicked in a sector window:

| Icon | Capture | Opens |
|---|---|---|
| System (top left, the factory) | `35-xyquine-system-window` | Xyquine's System window on its Manufacturing page (the three bands). |
| Missions (bottom right) | `37-mon-calamari-missions-window` | "Mission at Mon Calamari", its first mission selected in the red frame and the target and agent shown. |
| Defenses (bottom left) | `38-defenses-icon-fresh-personnel` | Mon Calamari's Defenses window on its Personnel page: the characters, then the special forces. |

Each window opens at the pointer, clamped into the galaxy view. A double click at (426, 269) or (382, 270) puts the 235 by 304 window at (305, 86): 540 − 235 by 390 − 304. That is `FUN_004593e0` passing its point to `FUN_0045aac0`, as in `personnel-states/29`.

When the icon's window is already minimized, the icon brings it back from the rail instead. It keeps its page (Trooper Regiments) and leaves an empty slot (`38-defenses-icon-restores-railed-window`). A minimized window is hidden, not destroyed.

## A difference fixed

The port forgot a minimized Defenses window's page. A restore from the rail reopened it on Personnel, and the icon's double click opened a second window while the rail kept the first. Now:

- the rail entry keeps the page (`RailEntry::Defenses`, `DefensesWindowAction::Minimize`);
- a restore shows that page again;
- the icon takes a railed window back where it stood, on its page (`SystemWindowState::take_railed_defenses`).

The unit test `a_railed_defenses_window_keeps_its_page_and_its_icon_takes_it_back` covers this. In the browser, `sector-quadrants.mjs`'s `defenses` case minimizes the window on Regiments, restores it from the rail on Regiments, minimizes it again, and asserts that the icon's double click brings it back on Regiments at its place with the slot emptied. It passes on Linux for both sides in `sector-quadrants-2026-11-03T22-14-30-937Z-3736641`.

## The Fleet window across the rail (2026-11-04)

**A0, day 353.** Pzob's Fleet window comes back from the rail as it was minimized: Fleet 6, the Personnel tab, Admiral Afyon (`39-pzob-fleet-restored-fleet6`). The Arrow is then selected on the left (`39-pzob-fleet-arrow-selected`) and the window minimized; it goes to the first empty slot, the eighth (`39-pzob-fleet-railed-slot7`). Restored, it shows the Arrow still selected (`39-pzob-fleet-restored-arrow`). That capture has the same SHA-256 as the one before the minimize: the window comes back pixel for pixel.

**The port, before.** A minimized Fleet window was dropped, and a restore opened a fresh one with nothing selected.

**The port, now.** `FleetWindowState::hide` keeps the window, and `open` brings it back with its selection, tab and place. The unit test `a_minimized_window_comes_back_with_its_selection_and_tab` covers this. `fleet-window.mjs`'s `chrome` case asserts that the restored window keeps its tab, its enabled tabs and its listed items. The fleet icon also takes a railed window off the rail (`take_railed_fleet`), by analogy with the Defenses icon. A0 did not show that for a Fleet window.

Run `fleet-window-2026-11-03T23-08-47-535Z-3850424`: pass.

Open: the Missions window’s rail entry keeps only its place; A0 has not been seen restoring one with its selection.

## The port

`sector-quadrants.mjs` runs each of these on Linux for both sides:

- **`open-system`** opens the System window and brings a railed one back with the icon.
- **`defenses`** opens on the personnel page at the clamped pointer.
- **`missions`** has the first mission selected in the frame (`FUN_0049f540`, `FUN_00609500`).

The last passing run is `sector-quadrants-2026-11-03T22-14-30-937Z-3736641`.
