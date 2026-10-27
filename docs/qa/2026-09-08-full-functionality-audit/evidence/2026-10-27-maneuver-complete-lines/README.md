# "Maneuver complete" needs a held target (P24)

Date: 2026-10-27. This is part of the review of the remaining `hyp:` tags.
Three tags in the tactical code are settled by static reading of
REBEXE.EXE. One of them changes behaviour.

## The original

### The fighter group's route end, `FUN_005c6610`

The objdump runs from `0x5c6a1f` to `0x5c6a83`. When the route empties,
the line is for the player's side (`+0x50 == [manager+0x9ec]`), and the
player's commander is not only observing (`cmp [eax+8], 1`). Then:
- `+0x60` is tested. Either way the force's targets (`+0x98`) are asked
  for their first item, through slot `+0x10` (`FUN_005f5060`).
- `je 0x5c6a9b` at `0x5c6a65` and at `0x5c6a78` skips the line when that
  list is empty.
- So `0x37`/`0xb1` plus the slot plays only for a group that holds a
  target.

### The capital force's route end, `FUN_005cc300`

- The same tests, at lines 205..235 of the decompile: `+0x60`, then
  `+0x98`, and a return when it is empty.
- The slot comes from `FUN_005a0300`:
  - a slot plays `0x2f`/`0xa9` plus the slot;
  - -1 plays the generic `0x2e`/`0xa8`.
- A loss uses the same pattern: `0x49`/`0xc3` plus the slot, or `0x48`/
  `0xc2`.
- Any route end gives the line, not only a maneuver's.

### The side's fighter list

`FUN_005ada10` asks each commander (`+8`, `+0xc`) for list `+0x64`'s first
item. `FUN_005d14d0` walks that list with the fighter fields `+0x164` and
`+0x170` (`FUN_005afe40`), and list `+0x58` with the ships. So `+0x64`
holds the side's squadrons.

## The port

- `advance_fighter_route` voices the line only while a member holds a
  target.
- `complete_force_waypoint` marks a player force complete at any route
  end where the ship holds a target (`attack_target` or `manual_targets`).
  It used to do so only at the end of a hook or a Hammer/Anvil.
- Comments settled: `task_force_slot` (the generic variants) and
  `side_has_fighter_groups`.
- hyp, narrower: a squadron inside a hangar is not in `+0x64`.

## Tests

- `a_fighter_route_ends_with_its_line_only_for_a_group_holding_a_target`
  fails without the change.
- `a_ships_route_ends_with_its_line_only_while_it_holds_a_target` is new.
- `a_force_s_completion_waits_only_for_its_own_members` and
  `waypoint_arrival_and_fighter_docking_complete_source_executors`: their
  arriving maneuver ships now hold a target, since the arrival's line asks
  for one (`FUN_005cc300`).

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2999 passed, 0 failed |
| `replay_manifest -- --ignored` (original data) | 2 passed; the golden is unchanged |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 7 tested: 7 caught |
| Release WASM SHA-256 | `d39e642bb4777d89216cabe351261fcb233d6ec4d5fe082ea38b4acb85a4f42b` |

Also settled, comment only: `TacticalTactic::source_code`. SetTactic
(`FUN_005c9be0`) writes `+0x38` and sets `+0x5c` to 2 only for code 2. A
new force (`FUN_005cbf50`) takes 2 with the Stand Off mover
(`FUN_005ec310`).
