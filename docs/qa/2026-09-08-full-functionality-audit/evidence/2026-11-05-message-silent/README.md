# The Message Index's posting toggle against A0 (2026-11-05)

This covers CMD-08-C013 ("silent").

## A0

The VM's Alliance save "test", day 8, with Advice Messages shown. The toggle (`0x66`, the window's (0x1a7, 0x93)) was clicked once, and the pointer moved away. Then it was clicked again.
- `message-silent/silent.png`: silenced. The art is 0x2ac7 (10951), keyed on its first pixel (`FUN_005fd170`), so its dark (35, 36, 35) border shows the rail behind it.
- `message-silent/alerting.png`: alerting again. The art is 0x2a78 (10872).

## Port

Both states were already right: `bmp_cache` keys 10951 on its first pixel. In `agent-advice.mjs` (Linux), the Alliance's toggle cell, 32 by 31 at the screen's (485, 196), is 0 differing from A0 in both states. The gate now asserts it. A run with an older screenshot (2026-11-01) had shown 125 differing pixels on the border, from before that key.

What the toggle does stays as before. It flips the shown category's option (`FUN_00467f10`), and a silent category's messages do not reach the advisor (`FUN_0048a060`). Unit tests cover both, and the gate checks the posting log lines (mask 0x200 silent=true, then false).

Gate run: agent-advice-2026-11-05T07-40-42-368Z, test WASM sha256 aa413a3bca5bdfb8….
