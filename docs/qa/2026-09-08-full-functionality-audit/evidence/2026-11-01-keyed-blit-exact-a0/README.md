# Exact keyed blits (2026-11-01)

## Claim

The original keys one exact palette index, its first DIB pixel's
(`FUN_005fd170` stores it, `FUN_005fc710` skips `*src == key`). The port
keyed its blue-screen resources with a range (r < 32, g < 32, b > 192), which
also removed the near blues (0, 0, 196), (0, 0, 223) and (28, 28, 210). It
also drew some Encyclopedia buttons unkeyed. Details:
`ghidra/notes/keyed-blit.md`.

## A0 (captures not committed)

- `.artifacts/a0-vm/status/01.png` (sha256 `4f59a2be3126232d95754dec106114864965d1d7c286f123ce28b53a3fb56e42`): 25 of the Empire cockpit
  901's 48 near-blue pixels show as drawn; the rest lie under animations.
- `.artifacts/a0-vm/encyclopedia/index-empire-day143.png` (sha256 `7bc6fb9ea8f1661c4bd1954732c30a6eec7869f8e3e4da85e763c4d8492715dc`):
  the Empire's rail button 10378 shows its near blues; 0x6f's corner (36, 118)
  and the 0x67 rail button 10380's corners (426, 89), (426, 126) are keyed.

## Port change

- `decode_rgba_image` keys exactly (0, 0, 255) for blue-screen resources.
- 10339..10344, 10380..10381 and 11615..11618 now key blue.
- Tests: `a_blue_keyed_bitmap_keeps_its_near_blue_pixels`,
  `the_encyclopedia_indexs_category_and_rail_buttons_key_out_blue`; both fail
  on the old rule. `gokres_miniature_blue_screen_becomes_transparent` expected
  (20, 20, 220) keyed, which FUN_005fc710's exact compare contradicts; it now
  expects that pixel drawn. 10380 left the opaque list of
  `message_index_rail_buttons_key_out_their_blue` on the A0 evidence above.

## Gates (Linux Chromium)

- `encyclopedia-index-shell` now blits button art keyed on exact blue. With
  the fix: 32 of 32 comparisons at 0 pixels (`gate-encyclopedia-index-shell.txt`).
  On HEAD's bmp_cache.rs: 37 and 83 pixels per comparison
  (`gate-encyclopedia-index-shell-without-fix.txt`). Before both changes it
  failed at 51 to 398.
- Every interface gate (`all-gates.txt`) passes except `encyclopedia-art`
  (needs REBELLION_EDATA_DIR) and `fleet-move` ("no bare map point lies within
  the pick radius"), both failing the same way before this change.

## Mutants

`--re 'decode_rgba_image|uses_blue_screen_transparency'`: 18 tested, 12 caught,
4 unviable, 2 missed. Both survivors are `&&` to `||` in the existing lime-key
test (944bdfca), not in this change.
