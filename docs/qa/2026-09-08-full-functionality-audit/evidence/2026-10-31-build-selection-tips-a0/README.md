# Build Selection tips and the tip font against A0 (CMD-11)

Date: 2026-10-31.

## A0

The VM's Alliance Easy game, day 13 to 30 (`.artifacts/a0-vm/build-selection`,
hashes in `a0-captures.sha256`):
- Yavin has no producer, so its bands' Build is disabled (02 to 04).
- Kimanan (Orus) has one construction yard. Right-clicking the Facilities
  band and choosing Build opens Build Selection at (192, 84) (08 to 10).
- `tip-*` rest the pointer on each control:
  - Edit Number, Items to Build, Increase, Decrease, Encyclopedia, Cancel
    and Close each show a tip at the pointer plus (0, 21).
  - The ink starts 4 pixels inside the left border, and the capitals 5
    pixels below the top.
- Kimanan had no room for a facility: every item read n/a, so Done stayed
  disabled and showed no tip (`tip-done`, 13, 14). An enabled Done's tip
  was not seen in A0.

## Findings and fixes

**Close tip.** `FUN_00438620` adds the close box through
`FUN_00606e10(0x277c, 0x277d, 100, 0x1903)`. The helper's fourth argument
is the tip, so the close box shows TEXTSTRA 0x1903 "Close", as A0 does.
`ghidra/notes/tooltips.md` had said the helper registers none, and the port
followed it. Both are corrected.

**Tip font.** The port's tips were 4 to 13 pixels wider than A0's, and that
had been put down to GDI hinting. The real cause: `win_tooltip::FONT_SIZE`
treated egui's size as the cell (12 × 2288 / 2048), but egui's size is the
em (epaint 0.31 `fonts.rs` 843 scales it by cell over em). The tips were
therefore drawn at em 13.4.

epaint rounds the cell to whole pixels, so em 12 is not available. The
14-pixel cell (em 12.53) is the nearer choice. Each tip in this session
and in A0 tooltip/01 and /02 now comes within 5 pixels of A0's width
(`tips_are_within_five_pixels_of_their_a0_widths`). With the old size it
failed: "Increase" was 56 against A0's 49.

With the smaller font the text line moved 1 pixel down (`TEXT_INSET`
3 × 3), so the capitals start 5 pixels down, as in A0.

`theme::original_font` already converts the cell to the em and is not
affected.

## Gate

`build-selection.mjs` now measures each tip's box from its bottom border and
compares it with A0. Left and top must match exactly; the width must be
within 15 %. The close box must show a tip. On the build without the fix it
failed (`close: false`). See the gate list below for the Linux Chromium runs.

All tip gates pass on Linux Chromium with the fix (`gates-summary.json`):
build-selection, tooltip, fleet-window, game-options, main-menu-rest,
personnel-orders, agent-objectives, status-window, encyclopedia-placement,
sector-quadrants, and `run.mjs --all` (152 cases).

`cargo mutants` has nothing to mutate here: the change is a constant, a
text inset and one table row.
