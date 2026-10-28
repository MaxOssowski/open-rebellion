# The Death Star's hangar: its squadrons fall with it and dock on it last (F-033)

Date: 2026-10-28. This settles the last open item of tracker item 2, "Wzory
walki kosmicznej": what fills the battle Death Star's lists `+0x38` and
`+0x3c` (`ghidra/notes/abstract-battle.md`, read 2026-10-22 in
`cdad7c5c`).

## The original

- The battle's Death Star object (`FUN_005a9220`, `FUN_005af4e0`, 0x7c
  bytes) builds its sub-object at `+0x1c` with `FUN_005b82a0`
  (`0x5af51d..0x5af527`: `lea edi,[esi+0x1c]; mov ecx,edi`).
- `FUN_005b82a0` is the hangar. A capital ship keeps the same object at
  `+0x418` (`FUN_005b05c0`; `FUN_005afe40` turns a hangar back into its
  ship with `-0x418`). Its fields:
  - `+4` carrier, `+8` squadrons inside, `+0xc` taken, `+0x10` capacity;
  - list `+0x1c`: docked (`FUN_005b8570`, `0x5b8592`);
  - list `+0x20`: inside, state 1 (`FUN_005b84a0`, `0x5b84c9`);
  - list `+0x28`: landing (`FUN_005b84e0`, `0x5b851c`).

  So the object's `+0x38` and `+0x3c` are the docked and inside squadrons.
- The capacity is the constructors' last argument (`+0x2c`, the hangar's
  `+0x10`). `FUN_0040bcd0` computes it from the Death Star's CapShip slots
  (`0x40bd8b..0x40be32`, reordered by `FUN_00595ef0` at `0x595eff`):
  - slot `+0x26c` (`0x500e40` -> `0x500630`): the class record's `+0xe4`,
    CAPSHPSD fighter capacity. The Death Star (id 136) has 24.
  - minus slot `+0x23c` (`FUN_00500fe0`): the squadrons aboard, iterator
    mode 3 over families `0x1c..0x1f` (`FUN_005039d0`);
  - plus those usable aboard (mode 1, `FUN_00513180`).
- `FUN_005afe40` docks a leaving squadron in this order:
  1. its home carrier (`+0x164`). When that is the Death Star's hangar, the
     test is `FUN_005ba420` (operational), else `FUN_005afc80`;
  2. the side's capital ships (`+0x58`). The Death Star is not one of them;
  3. for the Empire (`+0x168` = 1), the Death Star, while `FUN_005ba420`
     holds.
- On a hit, the trench run (`FUN_005aefc0`, `0x5af38b..0x5af3f4`) walks
  both lists, destroys each squadron (`FUN_005af7a0`) and frees the lists.

The note in `../2026-10-28-ctrl-clicks-a0/` that the Death Star's tactical
unit has no hangar was wrong; it now carries a correction.

The displayed battle's run (`FUN_005d04e0`) does not touch the lists; it
is unchanged.

## The port

`crates/rebellion-core/src/abstract_battle.rs`. The port's Death Star is a
capital ship of class 136, whose hangar already holds the class's fighter
capacity.

- `trench_run`: a hit destroys the Death Star ship with
  `AbstractBattle::destroy`, which destroys the squadrons in its hangar.
  Before this, only the ship fell, and its squadrons survived it.
- `leave`: a squadron tries its home carrier, then the side's ships other
  than the Death Star, and then the Death Star. A Death Star ship takes
  one only while the battle's Death Star is operational and Imperial.
  Before this, the Death Star stood in ship order, so a squadron could dock
  on it before another carrier with room.

## Tests

- `a_trench_run_that_hits_destroys_the_squadrons_inside_the_death_star`
- `a_leaving_imperial_group_docks_on_the_death_star_last`: another
  carrier with room comes first; the Death Star comes next; a destroyed
  Death Star takes none, and the group is destroyed.

Both fail without the change (`squad_count` stays 1; `([0], [], true)`
against `([], [0], true)`).

## Campaigns

24 dual-AI campaigns, 1500 days, against `50fc2152`: no change
(`campaigns.json`). The JSONL logs differ only in `wall_ms`. No campaign
had a successful trench run, or a retreat, with squadrons aboard the
Death Star.

## Gates

- Browser `--battle --all`: 148/148 passed (run `2026-10-28T13-03-23-488Z-2447050`).
- Workspace: 3016 tests passed. Provenance scan: no uncited items.
- Replay equivalence and the replay manifest passed; the golden is
  unchanged.
- Mutants (`abstract_battle.rs`, in the diff): the first run missed `&&`
  to `||` in `leave`'s `!alliance && is_death_star`. That test repeated
  `in_field`'s side test, so it was removed. Second run: 10 mutants, 9
  caught, 1 unviable.
- Release WASM SHA-256 `9eeaf9e7a88fe5f0e4b87eb34ddb5e5fa9990f76a061c3903ae5015f3597eafb`.
