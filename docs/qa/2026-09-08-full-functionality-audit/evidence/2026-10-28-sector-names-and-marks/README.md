# Sector window: names by side, a yellow title, headquarters and uprising marks (P06)

Date: 2026-10-28. This covers CMD-03's uncommon states (HQ, uprising) and
the sector window's text. The reading is in
`ghidra/notes/sector-quadrants.md`, "Names, title and marks in the sector
window".

## The original

- The title is the sector's name in font 4, yellow (`0x200f0f0`), centred
  in a 20-row box from (2, 2) (`FUN_00459e30`).
- Each planet's name is drawn in font 10 in its item's color
  (`FUN_004593e0`):
  - red for side 1 and green for side 2 (`FUN_0045bbb0`);
  - otherwise the constructor's white (`FUN_0060ba00`).
- A system in uprising gets its side's burning mark, STRATEGY 905 (side
  1) or 906 (side 2). It is keyed into the planet's picture.
- The Alliance headquarters (families `0x20..0x21`, the viewer's record)
  gets overlay 904, the floating headquarters, on its planet.

A0, Windows 10 VM, original `REBEXE.EXE`, Alliance player:

| Capture (`.artifacts/a0-vm/2026-10-28-nav/`, ignored) | What it shows | SHA-256 |
|---|---|---|
| `sector-a.png` | Moddell: yellow title; white names; Vjun red under its tracks, with the headquarters mark over the planet | `e88f5b40c9357721d95f9550e0cc5bdcdd68569c4f0753e5caf120577cbae017` |
| `adega-selected.png` | Moddell after a click on Adega: the name stays white | `b1982aaad8d11f0b7dc971954cb66fa04b418f64af0b88e34d26461bf9dbd2b0` |
| `sector-b.png` | Sesswenna: yellow title; Averam red; Coruscant and Bortras green; the explored neutral systems white; no mark on Coruscant | `4d4c3debd1080ef2cbf9a55a1aa6aa5b59663b53d35ab4fdaba3c189d6e445f9` |

The code also has a cyan branch for other sides, but the A0 captures show
the explored neutral names white, although the palette holds cyan. The port
follows the captures. The branch's state check is not traced (see the
note).

## The port

Before this change:
- planet names were colored by the player's relationship (green own, red
  enemy, cyan neutral, yellow in revolt) in a 10-pixel proportional font;
- the title was colored green, red or yellow by which side held more
  systems;
- neither mark was drawn.

Now, in `crates/rebellion-render/src/sector_window.rs`:
- `system_name_color` takes the side colors, white otherwise; port: white
  for an unexplored system, as its overlays show no side. The font is font
  10.
- The title is `SECTOR_TITLE_COLOR` in font 4, centred in its box.
- `uprising_mark` paints 905/906 over the planet. `shows_alliance_headquarters`
  paints 904 over the planet before the quadrant icons. port: the
  headquarters is the Alliance-held `is_headquarters` system, and the
  Empire sees it only while it sees the Alliance's contents there.
- 904..906 key out their blue matte (`bmp_cache.rs`).

## Tests

New:
- `a_name_takes_its_sides_colour_and_a_neutral_or_unexplored_one_stays_white`
- `a_system_in_uprising_takes_its_sides_mark`
- `the_alliance_headquarters_shows_to_the_alliance_and_to_an_empire_that_sees_it`
- `the_title_is_yellow_and_each_name_takes_its_items_colour`

Replaced: `a_system_in_revolt_takes_the_uprising_name_colour`. It held the
port's yellow revolt color, which has no source and which the A0 captures
contradict.

## Browser

Arch Chromium 155, run through a temporary Linux manifest. The Quadrants
fixture now sets two more systems of the primary's sector: the Alliance's
headquarters, and a system of the player's in uprising. The fixture
reports their planet rects. The `icons` case compares:
- the uprising mark pixel for pixel (905 for the Alliance, 906 for the
  Empire);
- the headquarters mark pixel for pixel for the Alliance. For the Empire,
  which does not see it here, more than half of the pixels must differ.

| Capture | SHA-256 |
|---|---|
| `sector-quadrants-2026-10-28T15-19-44-301Z-2940996/alliance-icons/icons-screen.png` | `fc305e4d27b7ef62c0ee8f6d5a6b876d52b909b25e97438744c7430068749152` |
| `sector-quadrants-2026-10-28T15-19-44-301Z-2940996/empire-icons/icons-screen.png` | `9cc338a4d929f783112f535a715e909d1124a07e4ca8121a866849c938c25a07` |

On the Alliance capture, Fwillsving stands under the headquarters mark,
as Vjun does in A0. Ithor has the uprising mark. The names are red or
green by side, and the Calaron title is yellow.

## Gates

- Workspace: 3025 tests passed. Provenance scan: no uncited items.
- Mutants (`rebellion-render`, the diff):
  - the first run missed `||` to `&&` in `shows_alliance_headquarters`. Its
    `player == Alliance` arm repeated `opposing_contents_visible`, which
    holds for the Alliance's own system, so the arm was removed and the
    Empire's fog case was added;
  - the second run missed two mutants of the title's `2.0 + 10.0`. The
    title test now checks the centre;
  - final run: 22 mutants, 22 caught.
- Browser:
  - `sector-quadrants.mjs`: 12 of 12 (run
    `sector-quadrants-2026-10-28T15-19-44-301Z-2940996`);
  - `fleet-window.mjs` `chrome`: 2 of 2;
  - `run.mjs --all` (GID): 152 of 152 (run `2026-10-28T15-20-24-538Z-2944052`).
- rustfmt: `sector_window.rs` keeps only its 3 baseline differences.
- Release WASM SHA-256 `96da0b51877a89ec439c11071954a636adf96483a8d8b70381790b9489674c51`.
