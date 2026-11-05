# One message shown (mode 2) against A0 (2026-11-05)

This covers CMD-08-C009 ("event"): story and event messages, shown one at a time.

## A0

- **`messages-day633/capture-palpatine`** (day 633). "Emperor Palpatine Captured" over STRATEGY 1049.
  - The title strip shows the item's image: 15 by 16, black, with Mission icon 0x2a9e at (1, 1) cut by its last column and row. It stands at the window's (15, 16).
  - The title, in font 4, has its ink from x 41 to 302, rows 12..28.
  - The body "Emperor Palpatine was captured by the Alliance at Corfai." has its ink from (18, 237) to (348, 249). "Emperor" is 48 wide here and in the title.
- **`message-rail/fleet-arrives-averam`** (a new Expert game, day 76). "Fleet Arrives at Averam" over picture STRATEGY 1018, every pixel. The Fleet icon 0x2ad4 (15 by 15), cut the same way, stands at (15, 15).

## The source

`Class_0065a1c0_vt44` (FUN_00468970), mode 2:
- It draws the chosen item's image (+0x20) through `FUN_005fc140(dc, 0x13, 0xf, 0, …)`. With no destination given, `SetDIBitsToDevice` puts the image at its own width and height, which explains (15, 15) and (15, 16).
- It then writes the title in font 4 from `FUN_00601b30(0x28, 0xd)`, (40, 13).

The image is the list item's unchosen image (`FUN_00468ab0`): the icon's size, black, with the icon at (1, 1).

## Before

The port drew no image left of the title. The title and the body were in a stand-in egui font (11 and 9 px), so the body's ink was about a third of A0's.

## Now

- **Title.** `title_icon_layout` places the image at (w, h) with the cut icon at (w + 1, h + 1). The title is in font 4 at (40, 13); the stand-in strip clip is gone, since the original writes from a point.
- **Body.** It is in font 4 with GDI word breaks (`paint_gdi_text_block`), the text control's (17, 234).
- **Tests.** `the_titles_image_stands_at_its_own_size_with_the_cut_icon` and `a_window_rectangle_lands_on_the_screen_scaled_from_the_origin` fail without the change. Mutants on the helpers: `on_screen` 7 caught and 4 unviable; `title_icon_layout` 5 caught and 2 unviable. The b75 in-diff run left 11 mutants in the draw code; the b76 refactor moved that arithmetic into the tested helpers.
- **`message-display.mjs`** (Linux, the Alliance):
  - The capture's title image is 0 differing from A0.
  - The title's ink box is [41, 12, 302, 28], A0's exactly.
  - The body's first line is [18, 237, 347, 249] against A0's [18, 237, 348, 249].
  - Fleet Arrives' picture is 0 differing from A0's 400 by 200.
  - The fixture files Fleet Arrives under Mission, so its title image is not compared.
- **Batches.** b75 and b76 each ran all 21 Linux gates, and every one passed. `agent-advice.mjs` passes on the b76 WASM.

## Still open

A long text shows the text control's scroll bar in A0 (`agent-advice/02-advice-message`, art 0x299a at the control's right edge, `FUN_0041ecf0` / `FUN_0041efe0`); the port has none yet.
