# Each withdrawing ship has a force of its own (P24)

Date: 2026-10-25. This follows `../2026-10-25-withdrawal-orders-queue/`.

## The original

For each capital ship of the withdrawing side, `FUN_005ad530` calls
`FUN_005aded0` (`0x005ad6ad`) before giving order 3. `FUN_005aded0`:

- takes the ship out of its task force (`0x005ade20` -> `0x005a8d30`
  with the force from `+0x178`);
- creates a new capital task force for it (`0x005cbf50`, `0xbc` bytes,
  vtable `0x66bf70`, with a Stand Off mover);
- registers it (manager slot `+0x4c`), posts event `0x30` and stores
  its id in the ship's `+0x178`.

The order goes to that new force (`0x005ad6b8`). `FUN_005cc300` tries the
jump of its force's first member (`0x005c6070`), and each force now holds
one ship.

## The port

- Before, `advance_side_withdrawal` let only the first member of each
  task force (by the port's group number) decide its jump. The others
  waited until the first had gone, and a first member that could not jump
  held the rest until the side's end.
- Now each withdrawing ship decides its own jump, as its own force's only
  member. `original_task_force_first_members` is removed.
- Not ported: the force object itself and its event `0x30`. The port's
  `task_force` stays the ship's interface group.

## Tests

- `a_task_force_jumps_its_ships_one_after_another` asserted the
  one-after-another order. It is replaced, on this evidence, by
  `each_withdrawing_ship_decides_its_own_jump`:
  - two ships of one group at their exit both decide at step 12 and both
    jump at step 24;
  - a ship that cannot jump holds no other.

  On the code before this stage the new test fails, because the second
  ship waits.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2794 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` | no mutants: the change only removes code |
| Release WASM SHA-256 | `c0d8ed83fe1cad2729809fc2f411ac5ad0eabd41ac1bae4c71086f56b0d46dc4` |

