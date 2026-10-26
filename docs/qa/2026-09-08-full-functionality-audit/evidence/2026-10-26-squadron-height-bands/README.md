# Squadrons step out of the ships' height bands (P24)

Date: 2026-10-26. Stage 6 of the fighter force mover. Follows
`../2026-10-26-fighter-speed/` and the capital version,
`../2026-10-26-height-bands/`.

Source: `0x5b8f70` (the fighter class `0x66c028`'s frame), read from the
disassembly; the decompile scrambles the flag variables.

## The original

1. The frame:
   - sets the floor `+0x404` to −FLT_MAX and the ceiling `+0x408` to
     +FLT_MAX, with the flags clear;
   - writes no blocked flag (`+0x414`);
   - does not return at speed 0.
2. After the turn and the alignment (`0x5b9102`), it searches with
   `FUN_005cdbf0`:
   - the capital tree only (`+0x8f8`); fighters never see fighters;
   - the squadron is put into the tree for the search at its x
     (`FUN_005ce950`, `FUN_005ee370`, ties by id);
   - the travel is speed × alignment × ms × 0.001;
   - its reach takes its last frame's velocity.
3. Floor and ceiling are built exactly as in the capital code, same-height
   ids included.
4. The y step is speed × 0.4 × ms × 0.001 (`DAT_0066c6b4`), with no
   alignment:
   - inside both: held. The fit path is dead, as in the capital code;
   - inside the ceiling: y −= min(y − ceiling, step);
   - inside the floor: y += min(floor − y, step);
   - otherwise y is untouched: no tracking of d.y, no limit, no overlap
     test.
5. Then the move: velocity = forward × speed × alignment, with forward.y
   included, and position += velocity × ms × 0.001.

## The port

- `original_height_band` now builds the band for both frames.
  `original_height_band_step` (capital) and the new
  `original_fighter_band_step` (squadron) use it.
- `nearby_capitals_of_squadron` is the squadron's search. It puts the
  squadron among the ships sorted by x, after ships of equal x.
- `fly_fighter_squadron` runs the band step between the alignment and the
  move.
- `port:`
  - the port's squadrons have no envelope, so their half-height and
    diameter are 0;
  - a squadron's id sorts after every ship's, for the tie and same-height
    rules.

## Tests

- `a_squadron_only_steps_out_of_a_height_band`: each branch. Outside both
  limits it keeps its y.
- `a_squadron_s_neighbours_are_the_ships_near_it`:
  - the stop;
  - the reach, including a ship exactly at it;
  - the y limit;
  - the reach growing with the squadron's last velocity.
- `a_squadron_s_neighbour_walk_off_the_origin`:
  - the ships' own velocity in their reach;
  - destroyed, jumping and envelope-less ships left out;
  - the walk past ships on both sides;
  - the vertical slack growing with the travel.
- `a_squadron_level_with_a_ship_steps_up`: a level ship goes below by the
  ids, for squadrons 0 and 2.
- `a_squadron_under_a_ship_steps_down_as_it_flies`.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2861 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T11-06-36-029Z-3604605`) |
| Replay equivalence | passed |
| `cargo mutants` | 146 tested: 102 caught, 1 unviable, 43 missed at first; after the new tests a targeted rerun left 19, and 2 of those then failed added cases by hand |
| Release WASM SHA-256 | `c3b35b0e9c2249fa454bc23e88f775ba3edadb5a055cf71ebb734e247a0c1a30` |

The survivors left are equivalent:

- **The dead "fits" path** in both steps, and the capital step's checks
  after its `dy == 0` return. Inside both limits the ceiling lies below the
  floor.
- **The search travel** (`speed × alignment × seconds`, 4 mutants). A
  squadron's half-height is 0, so any neighbour the extra vertical slack
  adds lies more than its own half-height away. The squadron cannot be
  inside that neighbour's band, so its y never changes.
- **The insertion's `>` with `>=`.** A ship of equal x is visited on one
  side or the other, and the walk goes on either way.
