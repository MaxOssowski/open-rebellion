# The Display pages' destination buttons: keyed, labels low (2026-11-07)

This covers EVT-03-C014 and EVT-03-C015 (the Assault Summary's go-to buttons) and TAC-07-C013 (the Battle Summary's).

## A0

The unedited "test" save (Alliance, day 720) listed "Assault on Yavin" on the Conflict rail. The Empire had taken Yavin. The captures are in `.artifacts/a0-vm/assault-clean/`:

| Capture | sha256 |
|---|---|
| `00-index-day720` | `6210b821eeec97451f0f55a44ac6fddfe1629cba6d38cb8f924a981f5067e975` |
| `01-summary-yavin` | `e02f54bed5766efc28b8ae7797019ff801b148970255b75d5c01acdb8b304346` |
| `02-attackers` | `3edfc71e707b6d11ebdf2d240fe43335160e318b50489dccdb180f4b5c422bef` |
| `03-system-forces` | `5880eddb99e39a6c9dd4c5cc0c6d892c7eac11be82503ae851834ce8b255cba6` |
| `04-display` | `32f96b504393f05e8084abd3c6c7b9eab750a27cd19045142ccb74211fc83c6d` |

The Display page (`04`) is the first A0 capture of the Assault Summary's Display.

## Two differences

**The keying.**
- The black of STRATEGY 10764 and 10765 (`0x2a0c`, `0x2a0d`) is transparent in A0. Their first stored pixel is index 0, black, so the strobe buttons key it.
- Under 10765's black, A0 shows the page: 1771 non-black pixels of the flare in `battle-clean/02`, and stars in `assault-clean/04`.
- The port drew both buttons opaque.

**The labels.**
- In A0 the labels' ink runs from row 79 to row 88 of the button: in the Assault Summary their line starts 76 below the button's top. The Battle Summary's labels sit at 96 (ink 99..108).
- The port drew the Assault Summary's labels at half the height less 12 (36), over the planet, in a 12 px font.

## Now

- `bmp_cache::uses_first_pixel_key` lists 10764 and 10765.
  - `the_display_pages_destination_buttons_are_keyed_on_their_first_pixel` checks this; it fails without the change.
  - The pressed 10767 has no capture and stays as it was.
- `assault_summary` draws each label with `paint_gdi_text` in font 4, centred, its line at the button's 76 (`GOTO_LABEL_TOP`). `display_goes_to_the_system_and_closes` checks each label's pen.
- **hyp.** `FUN_006027e0` gets `+0x34 / 2 - 12`, so `+0x34` would be 176. Not traced.

## Browser (Linux Chromium)

**`fleet-window.mjs` 'assault' (Alliance)** compares both buttons with A0 `assault-clean/04-display`, the window at (62, 49) in both:
- 0 differing pixels above the labels' line;
- each label's ink ends on A0's row 88.

The 'assault' case passes on both sides. Screenshot: `.artifacts/interface-parity/fleet-window-2026-11-06T21-59-35-655Z-1913402/alliance-assault/assault-display.png` (sha256 `dd16dc9b674c110ab7aee78d114157370e637f3d09c96efca061f19f9a02ff1f`).

**`run.mjs --battle --scenario=production-battle-results-presentation`:**
- `verifyStrategyBitmap` skips the black key for the two buttons.
- The new `battle-results-destinations-a0` probe compares both buttons with A0 `battle-clean/02`: system 0, fleet 0 differing pixels.
- Run `.artifacts/interface-parity/2026-11-06T21-53-36-858Z-1907236`: 4 of 4 passed.

## What the Fleet button does in A0

Pressed in A0, Fleet closed the Assault Summary and opened Yavin's Fleet window, which lists the Empire's Fleet 13 (`05-fleet-opens-yavin-fleet-window`, sha256 `cba740ee448c5950d5012496da86a19a373e85ca1c994569d819b03d4e3b1cdf`). The Sesswenna sector window stayed open beside it. That is `FUN_0045c8e0` kind 4 after `FUN_00429ce0`, as the port's `GotoFleets` does. The system button was then pressed too. The report reopened from the Conflict list (`06-reopened-summary`, sha256 `05fa80eb3a8fe220104a8baf2ab5da8513040a02c4b74a8b384dc5ec83e85d82`), then Display, then "Yavin": the summary closed and no window was added (`07-system-button-closes`, sha256 `76f0bca0c7504d9350b48dc2ae2be9b85ec0f2ec5e908e3081c6bd002655ef14`). Yavin's sector window and the Fleet window from the previous press were already open. That is `FUN_00429ce0` alone, as the port's `GotoSystem` does.
