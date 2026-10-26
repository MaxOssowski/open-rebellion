# The commander's first review regroups its ships (P24)

Date: 2026-10-26. Until now the computer side fought with all its capital
ships in one task force, slot 0, as they were placed. In the original, a
commander's first review rebuilds its capital task forces by armament,
range and speed.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository, checked against the `objdump` disassembly).

## The original

### When

In `FUN_005c8780`, when the commander is active (`+0x8` = 1: the computer's
side, or the player's while observing) and its count runs out (the first
turn, then every sixth):

- if its latch `+0x14` is clear, it sets it and calls slot `+0x18`
  (`FUN_005c8300`), before it links its targets (`FUN_005c9200`);
- so the regroup happens once per commander. The player's commander
  regroups at its first review under Observe.

### The lists, `FUN_005c81d0`

The battle's setup (`0x5abf57`) fills three lists from the commander's
capital ships (`+0x58`):

| List | Ships | Choice (`0x5c8296`..`0x5c82c2`) |
|---|---|---|
| `+0x98` | armed, without laser cannons | otherwise |
| `+0xa0` | with laser cannons | `FUN_005b7ce0` |
| `+0xa8` | unarmed | `FUN_005b7d10` = 1 |

- Each entry's key is `-(speed − longest range × -10000)`:
  - the speed is slot `+0x90`;
  - the longest range is `FUN_005b18d0`;
  - -10000 is `DAT_0066cbb0`.
- `FUN_005f4f10` inserts into a tree sorted by that key. An equal key goes
  after the existing ones, so the longest-ranged, then fastest, ships come
  first.

### The rebuild, `FUN_005c8300` and slot `+0x20` (`FUN_005c8660`)

- `FUN_005c8300` first posts event `0x32` (disband) for each of the
  commander's capital task forces (slot `+4` type 3).
- Then it calls the rebuild three times:
  - `k1 = rebuild(+0x98, 0, 3)`;
  - `k2 = rebuild(+0xa0, k1, k1 + 3)`;
  - `rebuild(+0xa8, k1 + k2, 7)`.
- For a list of `c` ships and slots `first`..`last`:
  - `k` starts at 1 and grows while `k ≤ last`, stopping at the first
    `k² > c`. Then `k` becomes `k − 1`, or 1 when `c < 2`;
  - `k` is capped at `last − first + 1`. So the first list makes at most
    3 forces, since the search stops at `k = 4`;
  - walking the list, the ship at position `i` starts a new force
    (`FUN_005aded0`) when `i % (c / k) == 0` and fewer than `k` exist. The
    other ships join the last one (`FUN_005ade50`);
  - for the local player, each new force goes to the next slot (`first`,
    `first + 1`, ...; `FUN_005a03f0`);
  - it returns `k`, or 0 for an empty list.
- Fighter groups are not touched.

## The port

- `BattleSession::commanders_regrouped` is each side's latch.
- `ai_side_leaves` calls `regroup_on_first_review` before
  `link_commander_targets`, for the computer's commander and for the
  observing player's.
- `rebuild_forces` follows `FUN_005c8660` and sets each ship's task force
  to its new force's slot. The key uses `original_tactical_mobility`'s
  engine power and `unit_max_weapon_range`.

`port:` choices:

- The lists are made at the first review, not at the setup. Nothing has
  fought by then, so the keys are the same.
- A force's slot is its task force, also for the computer's side.
- The old forces are empty once rebuilt, so their `0x32` events would
  change nothing. They are not posted.

## Tests

New, in `abstract_battle.rs`:

- `the_first_review_regroups_the_computers_ships`:
  - five laser ships make two forces (0-1, 2-4);
  - the armed list comes first, then the lasers, then the unarmed;
  - nine ships make three forces of three;
  - 25 ships make at most three forces, eight to a force;
- `the_regroup_puts_the_longest_ranges_first`: range before speed, also
  against a fast short-ranged ship;
- after the first mutation run, `the_first_review_regroups_the_computers_ships`
  also covers:
  - six ships (two forces);
  - sixteen laser ships after one armed ship, which make four forces in
    slots 1 to 4.

The tests use the new latch, so they do not compile without the change.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2898 passed, 0 failed (before the three added cases) |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T20-06-59-108Z-2700624`; Arch Chromium 155, not the pinned build) |
| Replay equivalence | passed |
| `cargo mutants` | 49 tested: 45 caught, 1 unviable, 1 timeout, 2 missed (first run: 5 missed) |
| Release WASM SHA-256 | `39f070e29f9dd065421e1d41a56b6045392746af6c087f0fdee98d2fa49667a2` |

The timeout is `+=` replaced by `*=` in the search for `k`: from 1 it never
grows, so the loop does not end. The two missed mutants are equivalent.
`count < 2` replaced by `==` or `<=`: for one or two ships the general
rule also gives one force.
