# The Death Star's destruction message takes the original texts (0x182)

Date: 2026-10-21 (overnight run, "Gwiazda Śmierci").

## The original

System Destroyed by the Death Star (`FUN_004950d0`) answers notification
`0x182`, then `2`, for both sides:
- the title is TEXTSTRA `0x70c8` "{2:1} Destroyed!" (the system);
- for a system a side held before the blast (`+0x64` 1 or 2), the body
  is `0x70c9` "The Death Star has destroyed the {1:4} system {3:1}.";
  the side's kind 4 is `FUN_0052d920`'s adjective;
- for a neutral system the body is `0x70ca` "The Death Star has
  destroyed the neutral system {2:1}.".

## What the port did before

The app posted "X DESTROYED by Death Star superlaser!", a port line.

## Port

- `story_messages::death_star_destroyed_texts` is `FUN_004950d0`.
- `destroy_planet` takes the system's holder before the blast and posts
  the original title and body for both sides. It is used by the Fleet
  menu's superlaser and by the Destroy System order.
- `apply_panel_action` passes the message texts through to it.

## Tests

- New: `the_death_star_names_the_side_whose_system_it_destroyed`.
- `cargo test --workspace`: 2504 passed, 0 failed.
- `cargo mutants --in-diff`: 7 mutants, 7 caught.

## Browser

- The Fleet window suite's `destroy-system` case now checks the body.
- Empire: "The Death Star has destroyed the Alliance system Akrit'tar.".
- Alliance: no Death Star, nothing destroyed.
- The results are in `destroy-browser.json` (0 errors), and the
  screenshot hashes in `screenshots.sha256`.

## Gates (`f29`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `a78f29ff8844d2c86a7275532c6607631610867e75dfbcb4f9b5abb41e60643a`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.

Hashes of the committed files are in `SHA256SUMS`.
