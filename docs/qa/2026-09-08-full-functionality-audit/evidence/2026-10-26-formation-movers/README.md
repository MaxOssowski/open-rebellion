# Formations move on their own anchors (P24)

Date: 2026-10-26. This closes the port's last tactical simplification:
"formations rigid on the lead unit". Before, the lead ship took the
waypoint, and after every move the members were put back at the lead plus
their slot. In the original, a force has a mover with its own anchor, and
every member, the lead included, steers to its place round it.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository, plus `objdump` where the decompile lost stack
variables). Floats are decoded from the executable.

## The original

### The mover

`FUN_005cb1a0` builds a capital force's mover at `+0x54`:

| Tactic | Constructor | Class | Step (secondary slot `+0xc`) | Turn (secondary slot `+4`) |
|---|---|---|---|---|
| Surround (1) | `FUN_005eb860` | `0x66d9a0` | `0x005eb540` | `0x005ead70` |
| Stand Off (2) | `FUN_005ec310` | `0x66da08` | `0x005eb1a0` | `0x005eae40` |

- Both constructors call the base `FUN_005ea030`. The anchor (`+0x3c`)
  starts at the lead unit's position. The forward (`+0x1c`) starts as
  (0, 0, 1) for a lead id below 1,000,000, else (0, 0, -1).
- The lead unit is member index 1 (`0x5ea250`). Every member's place is the
  anchor less its slot (`0x005eb040`).
- A tactic's event `0x3d` rebuilds the mover (`0x005c9c50` →
  `FUN_005cb1a0`), so a new anchor starts on the lead.

### Once a turn

`FUN_005ea740` (the mover's slot `+0x40`, called from `0x5a9d40` after the
bands):

- the speed `+0x124` is the slowest positive member speed (`0x5b17f0`), or
  0 when there is none;
- the turn rate `+0xa8` is the shortest member rate, starting from 99;
- `+0x134` counts the turns in a row with the anchor unchanged (compared
  with `+0xc8`).

### Each frame: the anchor

`0x5a9ec0` runs every force's order step (`FUN_005cc300`) before every
unit's move. `FUN_005cc300` sets the heading and calls the step with a
point and a radius:

- a route: radius 1.0;
- closing on a target: radius 2.0;
- the withdrawal: radius 1.0, with the forward and the heading set to the
  side's vector reversed.

The step (`0x005eb1a0` / `0x005eb540`, the same first half):

- the speed is the mover's `× 0.95` (`DAT_0066d84c`);
- within the radius of the point, the velocity becomes zero;
- beyond it, the velocity is the unit direction × speed, its y × 0.4
  (`DAT_0066d844`), and the anchor moves by velocity × ms × 0.001.

Then it turns (`FUN_005cd460`, at the mover's rate):

- **Stand Off** (`0x005eb1a0`):
  - a heading with no level part keeps the forward;
  - a heading behind flips the forward at once (all three axes), without
    turning the slots;
  - it turns only beyond π/12 (`DAT_006bd884`, set at `0x5e9de0`). The turn
    goes through `0x005eae40`, whose `FUN_005ead90` turns every slot's
    offset by the same angle;
  - when it did not turn and its velocity is zero, the velocity becomes
    (0, 1, 0).
- **Surround** (`0x005eb540`):
  - `FUN_005eae90` replaces the heading with the level direction of its
    last movement (`+0x68`). It falls back to the forward, then to the side
    vector, then to (0, 0, 1);
  - a heading behind flips the forward's x and z;
  - it always turns, but `0x005ead70` turns only the forward. The 22 slots
    stay on the world axes.

### Each frame: the members

`FUN_005b2f30` (each unit's slot `+8`):

- **Speed.** The member's own speed; it does not move at all when that is
  0. When the mover's speed is lower, it takes the mover's. With the
  anchor's 0.95, members catch up.
- **Distance.** `d` = place − position (`0x5b4960` → `0x5cb050` →
  `0x5eb040`), or zero within 1.0 squared (`DAT_0066d850`).
- **Height.** The member moves up or down toward `d.y` by at most speed ×
  0.4 × ms × 0.001 (`DAT_0066c4c4`, `DAT_0066c4fc`). An overlap
  (`FUN_005b2e60`) undoes it.
- **Alignment.** `a = max(0, f.x·t.x + f.z·t.z)`, using last frame's
  forward and heading.
- **Snap.** When the level `d` is within one frame's travel and
  `a ≥ 0.707` (`DAT_0066c508`), the member steps onto its place.
- **Heading** (slot `+0x80`, `FUN_005b4d40`), made level:
  - the withdrawal's reversed side vector;
  - else `d`;
  - else, when the mover's velocity is zero and `+0x134 ≥ 5`, the bearing
    to its target turned by its strongest arc against capital ships × π/2
    (slot `+0x84`, `0x66c4bc` π, `0x66c4f8` 0.5);
  - else its forward.
- **Turn.** It turns (slot `+4`, `0x5b1410`) only beyond π/48
  (`DAT_006bc4b0`, set at `0x5b0c70`), at its own rate.
- **Move.** With `a` above zero and a level `d`, its velocity is its level
  forward × speed × `a`. An overlap after the move returns it to where its
  height move left it.

## The port

`BattleSession::force_movers` holds a `TacticalForceMover` per moving
force (`TacticalForceKey`: a group slot of a side, or a ship's force of its
own):

- `sync_force_movers` keeps one per force, rebuilt when the force is new
  or its tactic changed. The anchor starts on the lead unit.
- `update_force_movers` runs after `update_engagement_bands` once a turn.
  It sets the speed, the turn rate and the still-turn count.
- `advance_original_tactical_movement`:
  - steps every force's anchor (`step_force_anchor`) toward its lead unit's
    waypoint, with the radii above;
  - then moves every member to its place (`move_force_member`);
  - on arrival the route moves on (`complete_force_waypoint`).
- `advance_engagement_approach` measures from the mover's anchor.
- `refresh_original_task_force_geometry` no longer runs each frame; only
  the battle's setup uses it.

`port:` and `hyp:` choices:

- `hyp:` the mover's starting forward is the lead's side vector. The
  original keys it on the lead's id (below 1,000,000), which was not
  traced; side vectors fit A0 (`../2026-10-25-side-facing/`).
- `port:` the speed and turn rate are set when the mover is built, not at
  the next turn.
- `port:` the members keep their own orders and routes. On arrival only
  the members bound for the reached point move on.
- `port:` the withdrawal heading points at the side's exit line, which is
  the reversed side vector.
- `port:` the facing toward a target uses capital targets only. A fighter
  group's slot `+0x20` in that check is the pure-virtual stub.
- Not ported:
  - the nearby objects' height bands (`+0x404`, `+0x408`, `FUN_005cdbf0`);
  - the blocked-member detour of `FUN_005ea740` (offsets (20, 0, 0),
    (-20, 10, 5), (5, 0, 20), (0, -10, -20)).
- Fighter forces keep their own movement (`FUN_005c6610`,
  `../2026-10-25-fighters-closing/`).

## Changed expectations

These tests asserted the rigid model. They now assert the original's:

- `movement_uses_effective_power_alignment_and_millisecond_integration`:
  - a ship moves only toward its place, so the test gives it a waypoint;
  - the anchor's 0.95 step;
  - the snap;
  - the level move at speed × alignment;
  - the 0.4 vertical step.
- `movement_restores_previous_position_when_mesh_envelopes_overlap`: the
  snap that would overlap is undone; without the obstacle the ship reaches
  1.1875.
- `waypoint_arrival_and_fighter_docking_complete_source_executors` and
  `ordered_navigation_route_advances_without_losing_the_next_waypoint`: a
  point within 1.0 of the anchor is reached where the force stands. The
  fighter starts 2.5 from the carrier, as before.
- `task_force_followers_rotate_and_move_with_the_retained_anchor`:
  - the mover turns at its rate and its slot axis with it;
  - the lead snaps onto the anchor;
  - the follower steers toward its turned slot.
- `a_withdrawal_sends_the_fighters_to_their_carriers_and_away`:
  - the carrier turns at π/12 a second before it can move, so the unpowered
    wing now starts 30 away to stay "on its way";
  - when the test moves the carrier to its exit line, it moves the anchor
    too.

## Tests

New (abstract_battle.rs):

- `a_formation_moves_at_its_slowest_member_less_five_percent`;
- `a_stand_off_formation_flips_without_turning_its_slots`: the flip, the
  π/12 dead band with velocity (0, 1, 0), and a turn that turns the slots;
- `a_surround_formation_keeps_its_slots_on_the_world_axes`;
- `a_still_formation_turns_its_strongest_arc_to_the_target`: arc 1 turns
  the bearing by π/2, and arc 0 not at all;
- `a_new_tactic_rebuilds_the_mover_on_the_lead_unit`.

The new tests use the mover's types and fields, so they do not compile
without the change.

## Gates

Browser probes that asserted the rigid model were updated with the same
evidence (`tools/interface-parity/run.mjs`):

- `production-maneuver-movement-presentation`:
  - the fixture gives its ship a point straight ahead and the ship's
    force no other members;
  - the probe expects the 0.95 anchor step, the snap and zero velocity;
  - the side check expects only that ship to move.
- `production-command-progression-presentation`:
  - both fixture ships are ordered alone, in forces of their own
    (`FUN_005aded0`);
  - the arriving ship stands where it is (within 1.0), its heading its
    forward.

| Gate | Result |
|---|---|
| Workspace tests | 2836 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T01-22-12-753Z-2863877`) |
| Replay equivalence | passed |
| `cargo mutants` | 281 tested: 263 caught, 4 unviable, 14 missed (first run: 56 missed) |
| Release WASM SHA-256 | `17a0c96d9447967d1d17fe2316d84df4f40432ff36caa2f026a15b23b89376c5` |

Of the 14 missed mutants, one was a gap: `exit - z` with `exit / z` in the
withdrawal heading. A ship between the field's middle and its own exit
gets the opposite sign. `a_withdrawing_member_heads_for_its_exit` now
includes that case, and the mutant, applied by hand, fails it. The other
13:

- Equivalent:
  - `-forward.x` and the withdrawal's `-away.x`, `-away.y`: those
    components are always 0, so the change only flips the sign of zero;
  - `-away.z`: a withdrawing ship is a force of its own. Its mover's
    forward only feeds the Stand Off turn, whose heading is set equal to
    it, and the slot of a lone member is 0;
  - `<` with `<=` in the turn's sign, and `>` with `>=` against π/48:
    these differ only at exact equality;
  - `&&` with `||` in the member filter: `moving_forces` already leaves
    out dead and jumping ships;
- Covered only by the browser matrix: the two fixture functions
  (`configure_maneuver_movement_fixture`,
  `configure_command_progression_fixture`) and the fixture's waypoint
  arithmetic. Scenarios `production-maneuver-movement-presentation` and
  `production-command-progression-presentation` check them.
