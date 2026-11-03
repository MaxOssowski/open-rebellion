# The galaxy view: press and release, wheel, right drag (2026-11-03)

Surface ledger CMD-02 C027 (selection), C028 (fixed-map right-drag no-op) and C029 (fixed-map wheel no-op).

## A0

The A0 run used the Alliance's DS_flip_allian save, days 229 and 230, with no window open over the galaxy. The captures are in `.artifacts/a0-vm/gid-selection/`, with hashes in `a0-sha256.txt`.

- **`01-held`, `02-released`.** A left press on the Moddell cluster at (155, 123), held for a second, shows only the hover label "Moddell Sector". The release opens Moddell's sector window.
- **`03`, `04`.** Three wheel-up and five wheel-down notches (QMP `input-send-event`, `wheel-up` and `wheel-down`), with the pointer at (330, 230). The galaxy view's (60..540, 60..395) differs in 0 pixels before and after.
- **`05`, `06`.** A right press at (300, 200), dragged to (420, 280) and released. The same region differs in 0 pixels; no menu and no pan.

This matches `FUN_00422ce0`: it has no wheel case and no right-button drag, and it opens the sector on `WM_LBUTTONUP` (`FUN_00429ce0`).

## The port

`run.mjs` (Linux, both sides, 640 by 480 and 1280 by 800 letterboxed):

- **`pan`, `zoom`.** A right drag and a wheel notch leave the frame byte-identical. This was already gated.
- **`hover`.** New: a left press held over the probe system leaves the frame byte-identical to the hover frame, and the release opens the sector, so the frame changes.

Run `2026-11-03T09-58-25-989Z-2519713` (`--scenario=hover`, four views): pass. `system-released.png` shows the Sluis sector opened.

## The hover label (CMD-02 C026)

**A0.** `01-held`: with the pointer on the Moddell cluster, "Moddell" over "Sector" shows in (255, 255, 0). Its ink spans (135, 91)..(183, 116).

**The source.**

- `FUN_00422620` builds the galaxy view's items. `FUN_00441520` adds a sector item:
  - at the sector's map point: `0x15`/`0x19` added for side 1, `0x54`/`0x1b` otherwise, the GID content origin;
  - with a hit rectangle reaching 8 past its farthest system's point;
  - with the text name plus TEXTSTRA 5947 "\nSector", font 5, colour `0x200f0f0`.
- `FUN_00422ce0`'s `WM_MOUSEMOVE` stores the first item whose rectangle holds the pointer (`PtInRect`) at `+0x25c`.
- Its `WM_PAINT` draws that item's text at the item's place.

**The port before.** The port tracked the hovered system for input but drew no label.

**The port now.**

- `sector_item_rect` and `hovered_sector` compute the rectangle and the hovered item.
- `draw_sector_hover_label` draws the name and "Sector" in font 5, in the palette's yellow, from the item's place.

**Tests.**

- `a_sector_item_spans_its_point_to_its_farthest_system_and_holds_the_pointer`: the rectangle at scale 1 and 2, the `PtInRect` edges, the first item found.
- `run.mjs` `hover` (Alliance, 640 by 480) points at (155, 123) and measures the label's yellow ink: (135, 92)..(181, 115), against A0's (135, 91)..(183, 116). The check is left and top within 2, bottom within 2, width within 15%: the accepted text deviation. Run `2026-11-03T10-06-39-820Z-2527805`: pass.

**Mutants.** 25 caught, 3 unviable, 2 missed:

- emptying `draw_sector_hover_label` survives the unit tests, but the `hover` gate catches it, because no label is drawn;
- the text box's width at other scales (`200 * scale`) does not change a one-word name's layout.
