# Capital escorts close on their ship, and keep their order when it is lost (P24)

Date: 2026-10-26. This follows `../2026-10-26-fighter-escort/`, which did
the same for fighter groups. Two port inventions are removed:

- a capital ship on Escort flew to an offset of its own round the escorted
  ship (8 to the side, 6 to 12 behind);
- when the escorted ship was lost, the port cleared the order, so the
  display read "No Orders".

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

### Closing on the escorted ship

- Escort (order 1, `FUN_005ca6d0` case 1) keeps the force's target list
  `+0x8c`, and `FUN_005cb600` puts the escorted ship there. Its executor
  (`0x66ceb8`) is empty.
- The capital force step `FUN_005cc300` has one target section for
  `+0x8c`, whatever the order. It reads the band (slot `+0xc`,
  `FUN_005edf80`), measures from the mover's anchor and steps the anchor
  toward the target with radius 2.0 (`0x40000000`).
- So an escort closes on its ship exactly as an attack on that ship would.
  The capital mover README (`../2026-10-26-formation-movers/`) counted the
  escort among the 1.0 radius cases. That was the port's own escort
  waypoint; the original's target section uses 2.0.

### The ship lost: `+0xb0`

- In the same section, when `+0x8c`'s unit is out of the fight
  (`FUN_005ad870`) and `+0xb0` is clear, the step sets `+0xb0`.
  - For the player's side it calls `FUN_005aaf50`.
  - The "target lost" voice needs no executor (`+0x60` = 0). Escort has
    one, so it plays nothing.
- The order `+0x64` and the list stay. Only a new target (`0x5cb600`) or an
  added one (`0x5cb4d0`) clears the latch.
- An escort holds its fire anyway (`FUN_005b3a40`, `0x5b3ae5`;
  `../2026-10-26-weapon-turn-gate/`), so the force simply stands.
- The fighter step `FUN_005c6610` returns at the latch before stepping.

## The port

- `ship_force_target` gives a capital ship's `+0x8c`: the escorted ship on
  Escort, else the attack target.
- `update_engagement_bands`, `advance_engagement_approach` and the frame's
  closing radius use it. Escort is no longer left out of any of them.
- `advance_original_escort_movement` is removed.
- `refresh_original_escort_targets` keeps the order and the target while
  the target is a friendly ship, alive or not. A ship that is lost, or
  jumped out, is no longer reached by `attack_target_position`, so the force
  stands and the group's anchor no longer steps.

`port:` the original keeps stepping a capital force toward the lost ship's
last position. Since that ship no longer moves and the force is already
near it, the port holds the force where it is.

## Tests

New, in `abstract_battle.rs`:

- `a_capital_escort_closes_on_its_ship_like_an_attack`: the escort's
  waypoint equals that of an attack on the same ship.
- `an_escort_keeps_its_order_when_its_ship_is_lost`:
  - a capital ship and a group escort a lost ship. After 20 steps both keep
    Escort and their target;
  - the group's anchor has not moved, and the capital ship has no waypoint;
  - an enemy as the target still clears.

Changed, in `tactical_view.rs` (render):

- `direct_escort_retains_friendly_target_and_clears_when_it_is_destroyed`
  is now `direct_escort_retains_friendly_target_even_when_it_is_destroyed`.
  - It asserted the port's clearing of the order. By `FUN_005cc300`'s
    `+0xb0` latch the order and the target stay, and the force stands.
  - Its waypoint now comes from the band approach, with Surround and the
    escorted ship in a force of its own. It used the removed
    `advance_original_escort_movement`.
- For it, `update_engagement_bands`, `update_force_movers` and
  `advance_engagement_approach` are now `pub` instead of `pub(crate)`.

Without the change the first fails (the escort was skipped and had no
waypoint), and the second fails on the cleared order.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2896 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T19-47-23-723Z-2605325`; Arch Chromium 155, not the pinned build) |
| Replay equivalence | passed |
| `cargo mutants` | 16 tested: 15 caught, 1 unviable |
| Release WASM SHA-256 | `1435a7a12689a340dac70246558482423c4bfe45f7271005f4265072a6b65599` |
