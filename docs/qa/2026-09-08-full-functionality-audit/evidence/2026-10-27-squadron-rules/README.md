# Squadron seating, Move refusals and a lost carrier's squadrons (P24)

Date: 2026-10-27. This is part of the review of the remaining `hyp:` tags.
Three squadron rules are settled by static reading of REBEXE.EXE, and each
one changes the port's behaviour.

## The original

### Which ship a squadron boards

- `FUN_00552300` builds the legs of a move to a fleet (`0x5525e7..0x5526bd`).
  It weighs the fleet itself, then every ship of the fleet's side in child
  order (`FUN_00502e30(fleet, 1, side)`), each through `FUN_00552b10`.
- At `0x552c82`, a later candidate replaces the kept one only when
  `FUN_0054fbb0` finds it strictly better. That comparator ranks:
  1. no acceptance refusal (`+0xc`/`+0x10`);
  2. no room refusal (`+0x14`/`+0x18`);
  3. unlimited room (`+4`);
  4. more room left after the move (`+8`, `FUN_0054fca0`: room less
     demand).
- So a squadron boards the ship with the most fighter room left
  (`+0x26c` less the squadrons aboard, `FUN_00500b40`), the earlier ship on
  a tie.
- Characters have unlimited room on every ship, so they board the first
  ship. That matches the port's `seat_characters`.

### A squadron's Move onto a planet

- `FUN_004f9860` holds the squadron's only object checks: slot `+0x58`
  (`FUN_00503000`, the order kinds `0x200`..`0x204`), its side, not
  complete, destroyed, and en route.
- The move's `0x204` sub-order checks the destination in `FUN_0053d430`. A
  squadron is neither a fleet nor a regiment, so any system whose side
  bits (`+0x24 >> 6 & 3`) differ from the mover's side is refused with
  `1`/`0x28`. A neutral or unheld system is refused too.
- The port refused only a planet held by the other side.

### A lost carrier's squadrons after a battle

- `FUN_0040c780`'s commit gives each squadron its parent (`FUN_0040d320`):
  - with its carrier flag set, its carrier, when the squadron is in that
    carrier's hangar or docked (state 1 or 5);
  - otherwise the system (`0x40d47c`).
- `FUN_005f5500` still finds a destroyed unit. Unit death (`FUN_005b2d20`)
  only takes the unit off its side's lists and sets state `0xb`; the
  registry tree at `+0x14` is cleared only whole.
- A dying carrier first kills its hangar's squadrons (`FUN_005b8d80` at
  `0x5b2dde`).
- So a lost carrier's hangar and docked squadrons die, and its launched
  ones go to the system. The port seated them on another ship of the
  fleet.

## The port

- `squadrons::seat_squadrons` seats each squadron on the ship with the
  most room left, the earlier one on a tie.
- `squadrons::check` refuses a planet unless its side is the mover's
  (`ControlKind::faction`).
- `tactical_flow::apply_results`: for an entry whose carrier was lost, the
  squadrons in its hangar or docked die, and the others go to the battle
  system's planet.

Not exact:
- hyp: the abstract battle's commit and the scrap still seat a lost
  ship's squadrons on another ship. The abstract result (`SpaceCombatResult`)
  does not carry the hangar states, and whether a scrap reaches
  SetDestroyedReason is open.
- Untraced: whether the demand adds up across the members of one move
  group (`FUN_00552dd0`), and whether delivery and seeding use this leg
  builder.
- Open: the commit's general rule sends every launched squadron to the
  system, the surviving carriers' too. The order of the commit against the
  registry teardown is not traced, so that rule is not applied to
  surviving carriers.

## Tests

- `squadrons_board_the_ship_with_the_most_room_left` (renamed). It adds two
  squadrons on two ships of room 3, which spread one a ship.
- `the_move_check` cases: a neutral planet is refused, the mover's own
  side's is allowed.
- `a_lost_carriers_hangar_squadrons_die_and_its_launched_ones_go_to_the_planet`
  fails without the change.
- `a_lost_ships_squadrons_board_another_ship` keeps the remaining hyp, with
  the new seating.

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 3000 passed, 0 failed |
| `replay_manifest -- --ignored` (original data) | 2 passed; the golden is unchanged |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Browser `production-battle-results-presentation` after the last edit | 4 passed |
| Replay equivalence | passed |
| `cargo mutants` | core and app, 25 tested: 22 caught, 1 unviable, 2 missed. The test then gained a surviving carrier and a lost entry with none out; the redundant zero check went. Rerun on the commit code: 11 tested, 9 caught, 1 unviable, 1 missed, which that edit removed |
| Release WASM SHA-256 | `04158a4b007cd91aea7dc6c973051cb7ba6cb16367cae75c732bbdaf88c7ec5b` |

The first workspace run had one failure,
`tactical_space_results_preserve_exact_capital_and_fighter_roster_slots`.
The stage had added a `retain` that dropped emptied entries, but the
roster keeps its slots, so the `retain` was removed.
