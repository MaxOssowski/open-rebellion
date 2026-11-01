# Manufacturing facility menu against A0 (2026-11-01)

OBJ-14-C007. A0 `status/28-fac-menu.png` (Windows 10 VM, Alliance,
Coruscant's System window, Training Facilities page): the right click on
the player's training facility opens a menu of Encyclopedia, Status and
Scrap, all enabled.

## Port (Linux Chromium 155)

`build-selection.mjs` now ends each side's run with this menu: after
Stop it opens the System window's Training Facilities tab, right-clicks
the page's first cell (the fixture's training facility) and checks the
rows' labels and states (`row_states`): Encyclopedia, Status, Scrap,
all enabled. The rows come from `FUN_0051d990` (Encyclopedia, Status)
and the facility's one `0x200` Scrap (`FUN_005399e0`,
`scrap::check`).

The frame measures 90 by 61 pixels in both (A0 x 230-319, y 188-249;
the port 267-356, 173-234 for its own click point). Text and fill as in
`2026-11-01-object-menu-text-a0`; deviations accepted 2026-10-30:
anti-aliased glyphs and the fill's palette step.

Run `build-selection-2026-11-01T08-53-22-986Z-12009` (Arch Chromium 155.0.8059.84, not the pinned
build), test WASM `c870dd9d600c24f6f10927f070869b09f983e7a8cf2c203869849ba35be3643a`. The whole gate
passes on both sides, including the Build Selection window's pixel
compare that failed by 6 pixels on 2026-10-28.

```
393c30b8a449c87d4ae7dd55ec7d03ac0dfa3028688026581d026e9cb058b72b  alliance/facility-menu.png
697199d889c8ed91a670fcfa5f7796d507d115ca3b638ea839a8853557c42e3e  empire/facility-menu.png
abb0772541d31f296e6e9526d27547268283576f8e21a32250bef55cc707643e  28-fac-menu.png  (A0)
```

The fixture's production report gives the Training Facilities tab and
the page's first cell (`training_tab`, `first_cell`).
