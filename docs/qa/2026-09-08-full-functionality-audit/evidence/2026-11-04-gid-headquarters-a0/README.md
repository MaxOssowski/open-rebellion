# The headquarters burst on the GID (2026-11-04)

This covers surface ledger cell CMD-02 C021, "HQ".

## The original

`FUN_0042b330` sets each system's GID item. After the display's switch, it rebuilds the item's flags:
- It sets flag 8 when `FUN_00526cf0(system, 3)` finds a child of families `0x20..0x21`, a headquarters.
- `FUN_00526cf0` builds a walk (`FUN_00513050`) over the system's direct children in mode 3 (existing, `+0x50` bit 6), with no side filter. So the flag asks nothing of what the viewer knows.

`FUN_00426ee0` loads STRATEGY 10166 (`0x27b6`, a 17 by 17 white burst) at the view's `+0x2c8`. `FUN_0042cde0` draws an item in one of two ways:
- With flag `0x80` (the unexplored gate), it draws only the unexplored mark.
- Otherwise, with flag 8 set, it first blits `+0x2c8` keyed at the item's point (`+0x28`, `+0x2c`), then the mark at the same point.

## A0

The capture is `gid/hq-burst-alliance-day390` (Windows 10 VM, Alliance save, day 390, Popular Support). It shows the burst under the Alliance's largest mark 10146 at (147, 115), in the upper-left cluster. Compared as 10166 and then 10146, keyed at that point, 0 of 89 pixels differ.

The earlier Imperial captures (`gid/d-*`, day 209) show no burst anywhere. That fits the Alliance headquarters standing in a system the Empire had not explored, which draws only the unexplored mark.

## The port

Before this change, the port loaded 10166 under the names `GID_SPECIAL` and `GID_SELECTION` but never drew it.

Now `gid_headquarters_burst` (`rebellion-render/src/lib.rs`) gives 10166 for a system with `is_headquarters`, unless the item shows the unexplored mark. The galaxy view draws it before the mark at the cell's top left. It is also listed in `drawn_gid_markers`, for the shaded copy under a sector window.

That shaded copy centred each mark on the system's point, while the view hangs the marks from the 17 by 17 cell's top left (`FUN_0042cde0`). A 15 by 15 mark therefore stood 1 pixel right of and below the view's. It now hangs from the cell's top left too. The unit test `the_window_shows_the_views_markers_under_it_shaded` asserted the centring, which was the port's own layout with no source; its expectation is changed on this evidence.

Tests:
- The unit test `a_headquarters_system_draws_the_burst_under_its_mark_unless_it_shows_unexplored` covers the rule.
- The fixture's ready report now carries the primary system's point and the GID bitmaps drawn there (`primary_gid_marks`).
- `run.mjs`'s `headquarters` scenario at 640 by 480 asserts the burst and then the mark at the primary system. It compares their keyed pixels exactly:
  - Alliance: 10166 and 10146, 0 of 89 pixels differ;
  - Empire: 10166 and 10150, 0 of 59 pixels differ.

Runs:
- `.artifacts/interface-parity/2026-11-04T07-10-14-557Z-583684` (`run.mjs --scenario=headquarters`, Linux Chromium): pass.
- Full batch b52:
  - workspace tests 3349 and fixture tests 116 pass; provenance-scan is clean;
  - mutants: 9 caught, 4 unviable, none survived;
  - all 19 Linux gates pass, `run --all` and `sector-windows` (the shaded copy against A0) among them.
