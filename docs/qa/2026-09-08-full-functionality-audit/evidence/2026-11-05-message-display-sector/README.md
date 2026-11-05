# Display (0x67) against A0: the system's sector window (2026-11-05)

This covers CMD-08-C014 ("navigate to object").

## A0

A new Expert Alliance game on day 43. The captures are in `.artifacts/a0-vm/message-rail/`.
1. Simulating the Battle at Averam (`battle-averam-result`) filed "Enemy Mission Foiled" under Mission Messages.
2. With that message chosen (`display-mission-chosen`), the rail's Display button (0x67) is enabled.
3. Display closes the index and opens the system's sector window, Sesswenna, at (56, 36) (`display-sesswenna`). It does not open the System window itself.

## Port

The port already did the same. `MessageIndexAction::Display` opens the system's sector window (`open_for_system`) and closes the index. The comparison showed one more difference. The sector window's switch-side button (STRATEGY 10210/10211, 14 by 14 at (264, 37)) drew its green first pixel (0, 255, 0) at (264, 50). A0 shows the frame there, because the button is a CoolStrobeButton keyed on its first pixel. `bmp_cache` now keys it.

## Tests

- **Unit test.** `the_sector_windows_switch_side_button_is_keyed_on_its_first_pixel` fails without the change.
- **`message-display.mjs`** (Linux, the Alliance). After Display, the sector window's frame is compared with A0 `display-sesswenna` at its left edge (56..62 by 36..396) and its bottom (56..297 by 392..398). Both are 0 differing. The switch-side key pixel is 0 differing; before the change it was 1. The sector inside differs (Calaron, not Sesswenna), so it is not compared.

## Still open

Two port gaps remain. A message whose object is not a system is taken as its system. Kind 4's Encyclopedia target is not carried. No A0 capture shows either.
- **The b74 batch.** It ran all 21 Linux gates, and every one passed.
