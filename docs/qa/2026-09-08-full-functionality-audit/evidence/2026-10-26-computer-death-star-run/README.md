# The Alliance's commander sends a group on the Death Star run (P24)

Date: 2026-10-26. Follows `../2026-10-26-computer-superlaser/`. With this,
the commander review of `FUN_005c8780` is ported apart from its
first-review regroup.

## The original

The review's Alliance branch (`0x5c8c47`..`0x5c8cb0`) runs only when all of
these hold:

- the Death Star is in the battle (`FUN_005a9330`) and operational
  (`FUN_005ba420`: not destroyed, hull above 0, no action committed);
- the commander's `+0x18` is still 0;
- no trench run is under way (`FUN_005d0ae0`).

`FUN_005c7e10` sets `+0x18` when the commander takes the side, to whether a
run is already under way.

1. If `FUN_005ad930` holds, the commander takes the first fighter of its
   list `+0x64` and that fighter's group (`+0x174`). When the group's live
   order is not 6, it sets `+0x18` = 1 and gives the group order 6 with
   `SetOrder`.

   `FUN_005ad930` holds when:
   - the Death Star is attackable (`FUN_005ba450`);
   - no run is under way;
   - the Alliance has more fighter units in the fight (`+0x988`, by side
     `+0x168`) than the Empire;
   - no Alliance task force is on order 6 (`+0x68` or `+0x64`).
2. Otherwise, with no fighters (`+0x64` empty), a side that may leave
   (`FUN_005c7e80`) withdraws (`FUN_005ad530`).

## The port

- `BattleSession::alliance_raid_sent` is `+0x18`. `set_player_observing`
  sets it, when the player's side is the Alliance, to whether a run is
  under way.
- `death_star_run_wanted` is `FUN_005ad930`. `fighters_in_fight` counts
  the living, non-empty, non-jumping groups of a side.
- `commander_trench_run` runs at the end of every review, the computer's
  and the observer's. It gives the first in-fight Alliance group order 6
  and posts its executor (event `0xa`) at the step's stamp, through the new
  `post_order_executor_stamped`. Failing that, the computer's side
  withdraws when it has no fighter left.
- `port:` only the computer's commander withdraws here, as only it does in
  the review's strength test.

## Tests

- `the_computer_alliance_sends_its_first_group_on_the_death_star_run_once`:
  - three Alliance wings (groups 1, 1, 2) against one TIE group: both wings
    of group 1 take order 6, executor at stamp 3, once only;
  - no run in these cases:
    - with as many TIE groups;
    - with a shielded Death Star;
    - with an emptied wing, which leaves no majority;
  - with no fighters, the Alliance's ship withdraws, not the player's;
  - no withdrawal in these cases:
    - without a Death Star;
    - with a destroyed one;
    - where it may not leave;
    - when it has already left;
    - while it still has fighters;
  - observing as the Alliance while a run is under way marks the run sent;
    observing as the Empire does not.

The test needs the new state, so it does not compile without the change.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2850 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T08-01-51-138Z-1685360`) |
| Replay equivalence | passed |
| `cargo mutants` | 52 tested: 45 caught at first, 6 missed, 1 unviable |
| Release WASM SHA-256 | `01493efc5a71ddcc18dcc6fd4ad5e5f51b04411a38a4d996a164002b51014ae5` |

After the edge cases a targeted rerun caught 48 of 49. The last survivor,
the withdrawing side's sign, now fails the test: the assertion that the
Alliance's ship leaves was added afterwards, and the mutant, applied by
hand, fails it.
