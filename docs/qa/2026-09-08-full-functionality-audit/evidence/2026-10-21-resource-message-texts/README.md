# The resource message reads its texts from TEXTSTRA (0x155)

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## The original

New Resources / Resources Depleted (`FUN_0048e940`) answers notification
`0x155`, then `2`, for the holder's side:
- a gain takes the title TEXTSTRA `0x7194` "New Resources on {1:1}" and
  the body `0x7199` (energy) or `0x719a` (raw materials);
- a loss takes `0x7196` "Resources Depleted on {1:1}" and `0x7197`
  (energy) or `0x7198` (raw materials).

## What the port did before

The app wrote the four messages as English strings in its code, title
and body joined by a colon. The words matched the shipped texts, but they
did not come from the player's TEXTSTRA, and the title was not separate.

## Port

- `story_messages::resource_texts` maps the RESRCTB code to the title
  and body:
  - 1: raw materials depleted;
  - 2: energy depleted;
  - 3: new raw materials;
  - 4: new energy.
- The app files the message with its title on the Resource rail, as
  before.

## Tests

- New: `a_resource_change_names_what_was_found_or_used_up`.
- `cargo mutants --in-diff`: 9 mutants, 9 caught.

## Gates (`f24`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `7c80df0dcab7e8227e4399d39a1cb87989bba88b9a46bd99e6c50829215c458b`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- Not checked: the message in the browser.

Hashes of the committed files are in `SHA256SUMS`.
