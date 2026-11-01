# Fleet and ship Rename against A0 (2026-11-01)

A0: the Windows 10 VM, the Empire Death Star save on day 580,
Chandrila's Fleet window, Fleet 1 selected with Victory Destroyer 2 in
its Capital Ships list. Captures `.artifacts/a0-vm/menus3/40` to `47`,
hashes in `a0-sha256.txt`.

## What A0 shows

- `40`: the player's ship in orbit: Move, Confirmed Move, Create Fleet,
  Rename, Encyclopedia, Status, Scrap, all enabled (the port's rows and
  states; OBJ-14-C003, C020).
- `41`-`43`: the ship's Rename opens the edit on its name line under its
  picture in the right list, the whole name selected black on white,
  Fleet 1 still selected; typing "Rogue" and Enter renames it there.
- `44`-`47`: the fleet's Rename (its menu as `27`) opens the edit on its
  left entry, "Fleet 1" selected black on white from the entry's left;
  "Vader" and Enter rename it, the right panel's title too.

## Changes

The port's ship Rename selected the ship on the left list and put the
edit over that entry, emptying the right list. It now keeps the fleet
selected and opens the edit on the ship's row of the right list (the
item's name line, +37, 125 wide, centred, `ship_item_row`), the label
hidden under it. Both edits show the selected name black on white and
typed text white (the selection was egui's orange). The rename log
names the object (`object=ship`/`object=fleet`).

`fleet-window.mjs ship-rename` (new) and `rename` pass on Linux
Chromium 155 for both sides: the ship's menu rows as A0's, the edit on
the right list, Enter renames it and the list shows "Rogue". Run
`fleet-window-2026-11-01T13-13-02-769Z-440095`:

```
710c9cd50fb243b1cd53a2a136695d4d13482e3316bf0024654178879949c651  alliance-ship-rename/ship-rename-edit.png
fe7892562de59e98620e258f1b993c5df19143fb91d7f664d081e4de78025601  empire-ship-rename/ship-rename-edit.png
24c6ea4bca77e7f2aebb626486fc37ed0f3239587cce0c9d3bafcd4a58ecd1c3  alliance-ship-rename/ship-renamed.png
86ed9aeca37e5520052b246d6efe9bfcee904e23fe4cf55cf8b3a7fee77a38d0  empire-ship-rename/ship-renamed.png
0b1d39e5e1524f31a06e591fb9b78928ce48f50edda74b7887ccd2480d755efb  alliance-rename/rename-empty.png
55d76a0eb46f464080411fdf61183e1eada2b28b977b1d6caeab70911e98e66d  empire-rename/rename-empty.png
548182e12410e6cf2b352afb1e7275c6ce56a9e344c620a687af08355fd040c5  alliance-rename/renamed.png
b655555581cd6c239beb8308cdf797e2ed548aa624d968b7d5b61bfa22cc45dc  empire-rename/renamed.png
```

Tests: `a_ship_of_the_selected_fleet_is_renamed_in_the_right_list`,
`the_rename_field_shows_the_selected_name_black_where_the_name_was`.
Mutants: 38 caught, 1 unviable, 1 missed: `to > 0` for `to >= 0` in the
whole-selection test differs only for an empty name, which has nothing
to show in either colour.
