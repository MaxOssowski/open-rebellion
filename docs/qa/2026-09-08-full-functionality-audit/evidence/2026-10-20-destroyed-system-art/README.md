# A destroyed system shows the asteroid field (F-017)

Date: 2026-10-20 (overnight run, "Gwiazda Śmierci").

## The original

`FUN_0045bbb0`, the refresh of a system's item in the sector window, tests
the destroyed bit (`+0x50` bit 3) first. For a destroyed system it does
three things:
- it sets item state `0x40`;
- it replaces the planet bitmap with STRATEGY `0x2800` (10240, a 37 by 37
  asteroid field);
- it sets the name colour to `0x2ffff00`, the unheld colour.

It then skips the rest of the refresh. The port had always drawn the
planet.

## Port

- `sector_window::planet_art` returns 10240 for a destroyed system.
- The name takes the unheld cyan.
- Test: `a_destroyed_system_shows_the_asteroid_field`.

## Browser

The Fleet window suite's `destroy-system` case (Empire) ends with
Akrit'tar drawn as the asteroid field with a cyan name. The screenshot was
inspected; its hash is in `screenshots.sha256`, with the other
screenshots of both cases.

## Gates

These are the `f14` gates, shared with the tactical fixture stage; see
`2026-10-20-tactical-death-star-fixture/`.

- Release WASM SHA-256
  `ec8600a3010194561b796e218159eac68b8c24b57bb32e8b66fb827177dd5fa0`.
- `cargo mutants --in-diff`: 6 mutants, 5 caught, 1 missed. The missed
  one is the name's font size (`10.0 * layout.scale`), unchanged context
  next to the edit.

Hashes of the committed files are in `SHA256SUMS`.
