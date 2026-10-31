# Message Index black window background (2026-10-31)

## Claim

The Message Index window is 470 by 331 (`FUN_00466350`). Its base 10335/10336
keys out pure blue (palette index 252). The last row (y 330) is the only blue
that the rail (58 by 330) and the content pane do not cover. A0 shows that row
black, so the keyed base sits on the window's own black background.

## A0

- `.artifacts/a0-vm/modal2/17-msg.png` (sha256 `00e705e68acba35d5547f94bf3823e0dcd65e2704197350c04be2d0dcc49f284`, not committed): the
  window is at screen (125, 52); screen row 382 is (0, 0, 0) across all 470
  pixels of the window.
- The Encyclopedia index reuses 10335/10336 in a 470 by 330 window, so row 330
  falls outside it, and A0 `encyclopedia/index-empire-day143.png` shows the
  galaxy map there. The Encyclopedia is unchanged.

## Port change

`draw_shell` (crates/rebellion-render/src/message_index.rs) fills the window
black before the layers, as the Fleet Finder and Assault Summary already do.

## Gates (Linux Chromium, `.browser-linux.json`)

- `message-index-shell` now composes its expectation with the keyed blue
  turned black. With the fix: 24 of 24 comparisons, 0 different pixels
  (`gate-message-index-shell.txt`, "wasm_sha256": "6176833e8c9a960a1986893214fe5cf659e993d0b5d426857342b651a9af9280", run dir `message-index-shell-2026-10-31T21-41-15-048Z-575255`).
- The same gate on the test WASM built from HEAD's message_index.rs fails:
  470 different pixels in every comparison, which is exactly the last row
  (`gate-message-index-shell-without-fix.txt`).
- Unit test `the_shell_paints_the_windows_black_background_under_its_base`
  fails without the fill and passes with it.

## Mutants

`cargo mutants --re draw_shell`: 17 tested, 15 caught. The two survivors are
`&&` to `||` at lines 675 and 689 in the existing category press logic
(a2a9f609, 42a9346d), not in this change.
