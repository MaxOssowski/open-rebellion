# The Message Index's title, chosen category and rail (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy" (the Message Index around
the advice). This closes the open points of
`../2026-10-29-message-index-posting/`.

## The original and A0 (`01-advice-index.png`, `40b45302…`, window (125, 52))

- **Title.** Mode 1's paint (`Class_0065a1c0_vt44`) writes TEXTSTRA
  `0x8019` "Message Index" in font 11 with its cell at (15, 20). A0 shows
  it white: the "M" from x 141, its caps 76..90.
- **Chosen category.** The controls form a radio group (`+0x160`). The
  chosen one shows its pressed art: A0's Advice control matches the
  Empire's 10849 (`0x2a61`) exactly.
- **Display Message (`0x65`).** In mode 1 its art is `0x2a76` / `0x2a77`
  (Alliance) or `0x29f2` / `0x29f3` (Empire), with the disabled art
  `0x2ad1` / `0x2ad2` (`FUN_00468fb0`). A0's matches 10738 (`0x29f2`)
  exactly. Mode 2 uses `0x2884` / `0x288a` for the way back. With an empty
  list it is disabled (`FUN_004697b0`).
- **Display (`0x67`) and Compose Chat (`0x68`).** They sit at y
  `0xc9`/`0xcf` and `0xff`/`0x10a`. A0 shows both in their disabled art,
  `0x2ad0` (10960) and `0x2a81` (10881), exactly.

## The port (`message_index.rs`, `bmp_cache.rs`)

- **Title.** The title draws in mode 1 (`title_pos`, `original_font(11)`).
  Its width differs: 142 in Liberation Sans against 155 in A0's hinted
  GDI Arial, the same kind of difference as the Encyclopedia's text.
- **Chosen category.** `draw_shell` shows the chosen category pressed
  (`category_pressed`). The bare shell of the interface fixture has none
  chosen.
- **Rail buttons.**
  - `window_button_specs` gives Display Message its mode's art and leaves
    it out while the list is empty.
  - `disabled_rail_controls` draws the disabled art of Display and
    Compose Chat, and of Display Message for an empty list.
  - port: Display is never enabled, because the port's messages do not
    carry the kinds that enable it (3 and 5 with a target, 4).
  - hyp: Compose Chat stays disabled without a multiplayer session.
- **Blue key.** The new art keys out its blue mat (10870..10871,
  10959..10962, 10878, 10881).

## Browser (Arch Chromium, release WASM `8b89ecbe…`)

- `agent-advice.mjs` passed for both sides at `…T23-08-43-613Z`
  (`agent-advice-result.json`). The Empire's index screenshot is
  `3d13c2a6…`; the Alliance's is `5c5d3db4…`.
- Against A0, the Empire's window (470 by 331) differs in 10,524 pixels.
  2,553 of them are outside the list rows, mostly anti-aliased title and
  heading text and the galaxy behind the window's edges. The list rows'
  text and content differ in themselves: A0 has a later game's advice.
- `agent-objectives.mjs` passed at `…T23-09-06-326Z`.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `display_and_compose_chat_sit_disabled_under_the_toggle`, extended
    for the empty list;
  - `the_chosen_category_stays_pressed_and_a_click_chooses_another`;
  - `the_bare_shell_reports_a_clicked_category`.
- `the_chosen_bar_shows_its_top_left_and_the_heading_its_cell` now also
  checks the title's cell.
- Changed expectation, for `FUN_00468fb0` and A0: mode 1's Display
  Message art in `close_and_display_sit_on_the_rail_and_delete_shows_in_the_list_only`
  (`0x2a76`, was `0x2884`).
- `cargo mutants --in-place --in-diff`:
  - the first run (excluding the app's `main.rs`) found 71 mutants and
    missed 8;
  - after the new tests, `message_index.rs` alone gives 69 mutants: 64
    caught, 4 unviable and 1 missed;
  - the missed one turns `primary_down && over` into `||`, so a hovered
    control would show pressed. That only changes the art, which tests
    without textures cannot see.
