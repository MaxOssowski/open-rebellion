# The main menu at rest against A0 (PRE-02-C001, C005, C015)

Date: 2026-10-31.

## What A0 showed

The A0 main-menu frames are 14 captures taken right after the pointer
arrived at a control, before its tip (`.artifacts/a0-vm/tooltip/221..234
-mm-*-base.png`, Windows 10 VM). Each frame was matched control by control
against COMMON.DLL's bitmaps (keyed comparison on their opaque pixels).

Five buttons show a different frame of their animation in every capture,
with the pointer nowhere near them: Load (11243..11254), Exit
(11197..11210), Empire (11001..11014), Alliance (11031..11044) and Head to
Head (11151..11180). So do the two unselected difficulties. Empire and
Alliance stay in lockstep. The selected difficulty stays on 11273.

The port showed:
- each button's rest art, animating only under the pointer;
- the galaxy lever at 10001 where A0 shows 10003;
- the game-type label in a monospace face, with the second type named
  "Headquarters Only".

## The original (`ghidra/notes/main-menu-animation.md`)

- **Animated buttons.** `FUN_00405560` creates them with `FUN_00603870`
  (first frame, frame count, and as pressed art the id the port showed at
  rest).
- **One clock.** A single periodic `timeSetEvent` of 66 ms (`FUN_00603b90`
  -> `FUN_00601f40`) runs `LAB_004054a0`, which steps all eight animated
  buttons (`FUN_00603a50`). The frames loop forward, with no hover state:
  `FUN_006028c0` has no `WM_MOUSEMOVE`. The clock stops while the credits
  run.
- **Pressed and selected art.** `FUN_00602d30` paints the pressed art
  only while the button is held. A difficulty button is a toggle; its
  selected art is its pressed art.
- **Lever.** `FUN_00603cd0` holds its states in the order 0x2713, 0x2712,
  0x2711. Its position starts at 2, which command 0x6a maps to galaxy size
  1, the default (`FUN_00404c30`). So Standard shows 10003.
- **Game-type label.** A text object at (271, 374), 110 by 14, colour
  0x00ff00, centred. It shows TEXTCOMM 0x1060 "Standard Game", and command
  0x71 swaps in 0x1061 "Headquarters Victory" (`FUN_00406000`).

## The port (`main_menu.rs`)

- `texture_for(control, state, pressed, now)` loops each animated button on
  a 66 ms clock and shows its pressed art only while it is held.
- The selected difficulty keeps 11273 / 11275 / 11274.
- Credits shows 11271, and 11272 when pressed.
- The lever shows 10003 / 10002 / 10001.
- The label reads TEXTCOMM (`game_type_label`) in game font 8, green
  0x00ff00. A0's ink is 8 rows tall and 74 wide, which is the 12-pixel
  cell; it sits one pixel left of and below egui's centring.

The unit tests that pinned the old rest art (Empire 10009 at rest, the
hovered Quit frame) now pin the original's behaviour, with this evidence.

## Gate (Arch Chromium 155, Linux)

`tools/interface-parity/main-menu-rest.mjs --a0=.artifacts/a0-vm/tooltip`
passes (`main-menu-rest-result.json`). It masks two kinds of pixel:
- those that change between the 14 A0 frames or between four port frames
  (the animated buttons);
- the port's music toggle and Fleet Registry chip (DEV-UI-001,
  DEV-UI-011).

Of the remaining **278,351 pixels, 0 differ from A0**. The label's ink box
matches A0's on its left, top and bottom edges; its right edge is one
pixel wider (the accepted text-width deviation). The label's own glyphs
are smoothed, as all of the port's text is.

Without this change the gate fails: the lever region alone differs (10001
vs 10003).

## Still open

- Hover (C003): the original has no hover art, and the port now has none
  either. No browser assertion checks that hovering leaves the art alone.
- The other galaxy sizes (C007), the difficulties (C006) and the game types
  (C008) have no A0 capture after a click.
