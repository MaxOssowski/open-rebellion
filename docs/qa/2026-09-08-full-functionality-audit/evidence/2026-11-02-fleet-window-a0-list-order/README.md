# The Fleet window's order against A0 (2026-11-02)

Follows `2026-11-02-fleet-window-a0-split-join`, which left the list order
open. Same A0 captures (`.artifacts/a0-vm/fleet3/`, hashes there).

## A0

- `05`, `06`, `10`: a ship joined to a fleet is listed by age, not by
  arrival. Corellian Corvette 1, the older ship, comes before Medium
  Transport 1 in Fleet 6.
- `07`: Fleet 7, made last, is listed after Fleet 6.

## The original

The original keeps a container's children in an AVL tree ordered by
object key:

- `FUN_005f5100` rebalances the tree.
- `FUN_005f5060` starts a walk at the leftmost node.
- `+0x10` is the next node.

The iterators `FUN_00513120` and `FUN_005130d0` walk that tree. Fleet
`vta8` (`FUN_004feca0`) and the Fleet window's lists read it.

## Changes

1. **Ships.** A fleet's ships are ordered by `ShipInstance::id` after a
   join or a merge (`fleet_join::order_ships`). The id stands in for the
   object key. An unnumbered fleet keeps its order.
2. **Fleets.**
   - Each fleet gets a creation serial in `insert_fleet`
     (`FleetNames::serials`).
   - `movement::listed_fleets` orders by that serial. The slotmap key did
     not keep creation order: a disbanded fleet's slot is reused, so a new
     fleet could sort first.
   - The Fleet window no longer re-sorts by key.
3. **Save format.** It goes from v110 to v111 for the serials. Saves of
   other versions are rejected, as before (approved by the user,
   2026-10-30).

## Result

| Step | Before | After | Ceiling |
|---|---|---|---|
| `06` | 3996 | 1816 | 2000 |
| `07` | 3342 | 986 | 1150 |
| `10` | 2554 | 1250 | 1400 |

All six steps are now asserted. What differs:

- Text hinting.
- The new fleet's number: A0 "Fleet 6", the port's next free number.
- In `06`, A0's stale frame on Medium Transport 1. The list control's
  selection rules are untraced.

## Tests and run

New tests:

- `a_joined_ship_takes_its_place_by_number_in_the_fleet` (`fleet_join.rs`)
- `a_fleet_made_later_is_listed_after_the_older_ones_even_in_a_reused_slot`
  (`movement.rs`)

Both fail without their change.

Gates and mutants:

- `cargo mutants --in-diff` on the core and render changes: 16 of 16
  caught.
- Workspace: 3312 tests pass. Fixtures: 109 pass. No golden changed.
- `fleet-window-2026-11-02T02-57-39-965Z-3086922` (Arch Chromium 155): 79
  cases pass. Test WASM
  `8ff9f185838bb2111e67d5e351949baed22b703df90b1d2962d6c26c1621d4a6`.
- The batch's first Fleet window run failed on two ceiling constants my
  edit had dropped. They were restored and the gate rerun.
- The other gates pass.
