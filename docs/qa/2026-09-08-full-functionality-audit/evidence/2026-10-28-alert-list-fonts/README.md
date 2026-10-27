# The Battle Alert's lists use the original's fonts (P24)

Date: 2026-10-28. The user found the Battle Alert's force lists harder to
read than the original's: our text was greyish, thin and sat low in its row.

## The original

- `FUN_00450770` fills the list with three fonts (`battle-alert-window.md`):
  - the heading ("Alliance Forces", "Imperial Forces", "System Assets")
    in font `0xb`;
  - fleets, ships and the system's name in font 5;
  - squadrons, regiments, personnel and facilities in font 4.
- `FUN_0060eed0`'s table (`theme.rs`): `0xb` is a 24 px cell at weight
  400; 5 is a 16 px cell at weight 700; 4 is a 16 px cell at weight 400.
- GDI draws each line from the top of its row's cell. On the A0 capture
  `alert-imperial-forces.png` (SHA-256 `ebf1693a…`), "Imperial Forces"
  fills rows 96..113 of the list's first row (92..132), and "Fleet 1" is
  centred on 140 in the second row (132..172).

## The port

- `PageOp::Text` gained `bold`; the page drawer measures and draws with
  the bold face when it is set.
- `battle_alert_list` uses `BATTLE_WINDOW_FONT_B` for the heading and
  `BATTLE_WINDOW_FONT_4` for the other rows, bold for font 5. Its baseline
  is the row's top plus Arial's ascent over the cell
  (`usWinAscent` 1854 of 2288).
- Side by side with the capture, the heading spans 140 px against the
  original's 142, and each row's text sits on the original's line.
- The tests `the_battle_alert_list_rows_are_forty_high_from_83_40` and
  `the_battle_alert_list_scrolls_by_pixels_under_its_bar` had expected
  the old centred baselines. They now expect the capture's, and font 5's
  bold.

## Point 1 of the user's list

The user checked these on the original (2026-10-28):
- the damage mark on lost units in the results;
- the turn before the jump;
- the launches from the hangar.

The damage-mark comment no longer carries `hyp`.

## Gates

- Browser `--battle --all`: 148/148 passed.
- Workspace: 3007 tests passed. Provenance scan: no uncited items.
- Replay equivalence and the replay manifest passed.
- Mutants (31): 14 caught, 6 unviable, 11 missed. All 11 are in
  `draw_page_ops_at`'s macroquad drawing (centring the text), which no
  unit test reaches. The page's steps it draws are tested.

## Captures

Linux Chromium, 640x480, run `2026-10-27T23-34-23-136Z-591205`.

| Capture | SHA-256 |
|---|---|
| alliance `battle-alert-tab-2.png` | `1bdee65be241a6e0882a890b2402832969589a9e2ac0663977483df9c34aee88
bd55a56570c53326d7b8cc93f39b33a4e29ad82aa01adf16f1d6b0d9cb67a4fe` |
| empire `battle-alert-tab-2.png` | `1bdee65be241a6e0882a890b2402832969589a9e2ac0663977483df9c34aee88
bd55a56570c53326d7b8cc93f39b33a4e29ad82aa01adf16f1d6b0d9cb67a4fe` |
