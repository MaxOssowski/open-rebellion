# Fighter groups move as one force on an anchor (P24)

Date: 2026-10-26. This is the first stage of the fighter force mover. The
port moved each fighter squadron alone, as a point.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`, plus
`objdump`; outside the repository). The slot offsets come from an x87
trace of `0x5ebee0`.

## Corrections to earlier notes

- The fighter squadron's class is `0x66c028` (constructor `FUN_005b9c60`),
  with its own frame `0x5b8f70`. `0x66c560` is the abstract base class, so
  `FUN_005b2f30` is not shared with fighters, as
  `../2026-10-25-tactical-frame-movement/` had said. That note is now
  corrected.
- The fighter's move is the fighter mover's step `0x005ec0d0`, not
  `0x005eb1a0`. `../2026-10-25-fighters-closing/` is now corrected; the
  port's code had cited `0x005ec0d0` already.

## The original

- **One force per group** (`FUN_005adfa0`). The first squadron of the
  group becomes the lead (`FUN_005c62b0`); the rest join through force
  slots `+0x4c` and `+0x28`. The force class `0x66bee0` returns 4 from
  slot `+4`. A tactic's event `0x3d` rebuilds only class 3, so a fighter
  force keeps its mover.
- **The mover `FUN_005c72b0`** (vtables `0x66cb50` / `0x66cb30`, on
  `FUN_005ea030`):
  - its anchor `+0x3c` starts on the first member;
  - its forward is (0, 0, ±1) and never turns;
  - its speed `+0x124` is the slowest positive member speed, set once a
    turn by `0x5ea740`.
- **The step `0x005ec0d0`:**
  - beyond its radius of the point, the anchor moves by the unit delta ×
    speed × ms × 0.001, in three dimensions and unclamped, so it can
    overshoot;
  - within the radius it clears the velocity and returns 0.
- **The slots `0x5ebee0`** (slot `+0x30`) are set when a member joins, by
  the count `n` it makes:
  - the lead has none;
  - the others take unit vectors, in the plane for a forward of
    (0, 0, 1). They repeat every 30, and a forward of (0, 0, -1) negates
    them;
  - a member that leaves frees the count (`0x5ea630`);
  - a squadron's place is the anchor less its slot (`0x5ea500`).
- **`FUN_005c6610` targets on the anchor.** It steps toward a point one
  tactic `+0xc` step toward the target, with radius 2.0. On arrival it sets
  the anchor to the circling point.

## The port

- `TacticalFighterMover` and `BattleSession::fighter_movers`, one per side
  and group.
- `sync_fighter_movers` keeps them current, each frame and at the turn
  update.
- `ORIGINAL_FIGHTER_SLOT_OFFSETS` and `original_fighter_slot_offset` hold
  the traced table, which is a mirrored ring of unit vectors.
- `step_fighter_anchor` is `0x005ec0d0`.
- `advance_fighter_approach` now steps each force's anchor for its first
  free member, and circles it on arrival.
- `update_force_movers` sets the speed once a turn.
- `port:` a squadron's speed is still its sublight rating, not
  `0x5ba190`.
- `port:` until the squadron frame `0x5b8f70` is ported (stage 4), each
  free member stands at its place. Recovery, escort, withdrawal and the
  trench run keep their own movement for now.

## Tests

- `fighter_slots_follow_the_traced_offsets`:
  - the lead's zero offset;
  - n = 2, 3, 4, 5 and 8 against the trace;
  - the period of 30 and the negation;
  - the table's mirror property and unit length.
- `the_fighter_anchor_steps_in_three_dimensions_and_can_overshoot`.
- `a_fighter_group_moves_as_one_force`:
  - one mover per side and group;
  - the slowest member's speed, with a squadron without speed left out;
  - the places and lookups;
  - a freed count reused by a late member;
  - emptied squadrons and groups, on one side or both.

The new tests use the new types, so they do not compile without the
change.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2853 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T08-32-07-654Z-2003568`) |
| Replay equivalence | passed |
| `cargo mutants` | 152 tested: 113 caught at first, 36 missed; after the extra tests 143 caught, 3 unviable, 6 missed; a last case killed one more, checked by hand |
| Release WASM SHA-256 | `dff225489cd3b97de3862c27e6ea2481286411dd30ced41e40428d1be085ae73` |

The 5 survivors left are equivalent:

- `< 0.0` with `<= 0.0` on a forward z that is always ±1;
- multiplying by that sign of ±1 with dividing by it (3 mutants);
- the place's z, `anchor.z − offset.z`, with `+`, when the offset's z is
  always 0.
