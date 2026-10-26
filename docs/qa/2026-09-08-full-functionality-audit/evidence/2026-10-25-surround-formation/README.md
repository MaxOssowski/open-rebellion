# The Surround formation (P24)

Date: 2026-10-25. Until now the port placed every task force by the Stand
Off slots (`FUN_005ec400`), whatever tactic it was given. The trace is in
`ghidra/notes/tactical-collision-formation.md`.

## The original

SetTactic (`0x005c9be0`):

- stores the tactic in `+0x38` at once;
- posts event `0x3d` with the command stamp.

On its turn, the handler (`0x005c9c50`):

- builds the tactic object `+0x40`;
- for a capital force (type 3), rebuilds the mover with formation 1
  (Surround) or 2 (Stand Off) through `0x005cb1a0`.

A force created by `0x005cbf50` starts with a Stand Off mover
(`0x005ec310`).

The Surround mover (`0x005eb860`) adds the lead unit first, as index 1,
and the other members after it (`0x005ea250`). Each one's slot is
`0x005eb8d0(index)`. That function was run in an emulator for indices
0..45:

- a world-axis direction from a 22-entry table (`index % 22`; cases 15
  and 19 are the same), normalized;
- times `(floor(index / 22) * 0.25 + 1) * 12.5`. `DAT_006bd88c` is 12.5,
  from 5.0 × 2.5 (`0x005e9db0`).

A member's place is the anchor less its slot (`0x005eb040`). The anchor
is the lead unit's position when the mover was built.

## The port

- `BattleSession::task_force_formations[side][slot]`: Stand Off unless a
  Surround tactic has run.
- Committing a tactic for a force in a group slot posts
  `TacticalTurnEvent::TaskForceTactic` (`0x3d`) at the command stamp. Its
  turn sets the formation.
- A force formed from a selection, or by assigning to a blank slot,
  starts Stand Off.
- `refresh_original_task_force_geometry` places a Surround force's members
  at `lead + slot(1) - slot(k)`, with k = 2, 3, ... in the capability
  order.
- `port:` as with Stand Off, the formation stays rigid on the lead unit.
  (Superseded in `../2026-10-26-formation-movers/`: the anchor now moves
  and the members steer to their places.) The original's formation step
  (`0x005eb1a0`) moves its anchor and
  members over time; that step is not ported.

## Tests

- `the_surround_slots_follow_the_original_table`: slots 0, 1, 2, 3, 7, 21,
  23 and 45 against the emulator's values; 15 equals 19.
- `a_surround_force_places_its_members_by_their_slots`: lead (10, 2, -40),
  second member (10, 2, -15), third (-2.5, 2, -27.5); the other side's
  slot is unaffected.
- `the_surround_slots_follow_the_original_table` also checks all 22
  entries at length 1, as the emulator printed them.
- `a_ship_assigned_to_a_task_force_takes_its_formation_place`: a ship
  joining a force takes its place at once and keeps the force's
  formation; a blank slot starts Stand Off.
- `a_force_s_formation_changes_on_its_tactic_turn`: the formation is
  still Stand Off through step 15 and becomes Surround at step 16; a
  newly formed force starts Stand Off.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2805 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` | 77 tested: 47 caught at first. The table signs and the vertical slot were then covered; a rerun missed 11, the blank-slot reset among them, which a further test closed; 10 remain, explained below |
| Release WASM SHA-256 | `f6e99bafafcecd5d98c78218e9da938dc4f7e6c92e011c2b7a33f7046c610f3a` |

The remaining mutants:

- `+ lead_slot.x` and `+ lead_slot.y` turned into `-`: the lead slot (index
  1) is (0, 0, 12.5), so its x and y are 0 and the sign does not matter.
- `assigned > 0` turned into `>= 0` before the geometry refresh: with
  nothing assigned the refresh changes nothing.
- `draw_tactical_view` and `apply_original_collision_envelopes`: the frame
  and envelope paths, whose refresh call this stage only re-wrapped. The
  browser matrix covers them.

