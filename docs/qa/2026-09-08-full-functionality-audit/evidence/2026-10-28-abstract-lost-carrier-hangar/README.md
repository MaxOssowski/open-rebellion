# A carrier lost in the abstract battle takes its hangar (P24)

Date: 2026-10-28. This closes the last squadron hypothesis: how the scrap
and the abstract battle treat a lost ship's squadrons.

## The original

`FUN_005af7a0` destroys a unit (state `0xb`). For a capital ship (slot
`+0x20` is 0) it then walks the hangar lists `+0x434` and `+0x438` and
destroys each squadron inside.

The abstract battle calls it:
- in `FUN_005d14d0`, on a target whose hull (`+0x3cc`) is 0 after its
  damage (`FUN_005b54d0`);
- for the Death Star's shot and a destroyed side;
- in the side's leave (`FUN_005d14d0` step 5).

## The port

The port's abstract `destroy` set the ship's state alone, and `deal` never
called it. So a squadron in a carrier lost to damage, or to the Death Star,
outlived it, was counted as a survivor and was seated on another ship. Only
the leave destroyed a hangar.

Now:
- `destroy` destroys the squadrons in a capital ship's hangar;
- `deal` destroys a capital ship whose hull reaches 0.

Test: `a_carrier_lost_in_the_abstract_battle_takes_its_hangar_with_it`, by
damage and outright. A squadron already out stays. The test failed before
the change: the hangar was kept with its 12 craft.

## Where squadrons now go when their ship is lost

| Path | A squadron inside | A squadron out |
|---|---|---|
| Scrap, or a ship destroyed outside a battle | dies (`crate::scrap::destroy_ship`, `GameObj_vt11c`) | — |
| The abstract battle | dies (`FUN_005af7a0`) | stays in the fleet |
| The 3D battle | dies (the tactical commit) | stays in the fleet (A0 VM, Yavin) |

port: what puts a squadron that stayed out back aboard a ship is untraced.
`number_ships` seats it on the ship with the most room. The
`squadrons.rs` test comment now carries this as `port:`.

## Gates

- Browser `--battle --all`: 148/148 passed.
- Workspace: 3012 tests passed. Provenance scan: no uncited items.
- Replay equivalence and the replay manifest passed.
- Mutants: 3 in all, 3 caught.
