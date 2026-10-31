# Command-centre controls against A0 (CMD-10)

Date: 2026-10-31.

## A0

In the VM's new Alliance game (`.artifacts/a0-vm/controls`, hashes in
`a0-captures.sha256`):
- **Game Options globe.** Held, it shows 10013 (`globe-held`); released,
  it opens Game Options (`globe-options`).
- **Finder controls.** The Fleet, Troop and Personnel Finder controls at
  (170, 415), (222, 413) and (271, 413) open their Finders
  (`*-finder.png`).

## Finding and fix

Beside the pointer, the held globe differed from the port's in one pixel:
its bottom-left corner. The command centre's controls are CoolStrobe
buttons, which `FUN_00427270` makes and which blit keyed on their first DIB
pixel: magenta for 10001..10028, except 10007 and 10008, which are grey.
The port drew each control's magenta corner. STRATEGY 10001..10028 now key
on their first pixel (`bmp_cache::uses_first_pixel_key`).

## Comparisons after the fix (Linux)

- **Pressed globe.** The GID gate's `game-options-pressed` frame matches
  `globe-held`; leaving out A0's pointer, none of the globe's pixels
  differ.
- **Empire controls.** Against A0 `gid/d-popular-support.png`, the bottom
  controls (140..560 by 430..460) and the globe (79..114 by 192..249)
  differ in 0 pixels.
- **New game.** A new Alliance game's first frame against
  `new-alliance/t18` differs in 1,884 pixels, down from 2,567. The rest is
  the droids' frames, the maintenance figure and A0's pointer.
- **Finder controls.** `audit-batch.mjs` now clicks the three Finder
  controls on both sides and checks that each opens its Finder
  (`audit-batch-controls.json`). The Finders themselves match A0 through
  F3..F5 (`encyclopedia-placement.mjs --window=finders`).
- **The rest.** The GID catalog (152/152) and `briefing.mjs` pass.
