# A leaving side's fighters dock on a carrier with room (P24)

Date: 2026-10-27. The rule here was tagged `hyp:`. When a side left a
simulated battle, each fighter group without a hyperdrive escaped if any
carrier of its side escaped, with no check of room or of which carrier.
The port also took the ships before the fighters. A squadron inside a lost
carrier's hangar survived.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`, outside
the repository), with objdump where the decompile lost `ecx`.

## The original

### `FUN_005d14d0` step 5

It walks the side's lists in this order:

1. The fighter groups, `param_1[0x19]` (`+0x64`). For each:
   - with a working hyperdrive (`FUN_005b1b70`), `FUN_005af910` (state 9);
   - without one, `FUN_005afe40`, and if that fails, `FUN_005af7a0`.
2. The capital ships, `param_1[0x16]` (`+0x58`). For each:
   - with a working hyperdrive, `FUN_005af910`;
   - else `FUN_005af7a0`.

### `FUN_005afe40` (a group without a hyperdrive)

- It needs the group in state 2 (`+0x170`, in the field).
- It tries the group's home carrier first (`+0x164`, through
  `FUN_005a6360`). `FUN_005afc80` needs that ship to exist and to be in
  state 2. `FUN_005b84a0` docks the group only if the hangar's `+8` is
  below its `+0x10`; docking sets the group's `+0x164` and state 1, and
  raises `+8`.
- Then it walks the side's ships (`[side + 0x58]`) with the same two
  tests.
- For side 1 it finally tries the Death Star: `FUN_005a66a0`, then
  `FUN_005ba420`.
- Success sets the group's state to 5.

### `FUN_005af7a0` (a destroyed unit)

It sets state `0xb`, then destroys every squadron in the hangar's lists
(`param_1[0x10d]` and `[0x10e]`, `+0x434` and `+0x438`, the hangar
`+0x418` plus `0x1c` and `0x20`). So a group that boards a carrier that
cannot jump is lost with it.

## The port

`abstract_battle::leave` follows that order:
- the groups dock in `TacticalShip::hangar`, home carrier first, through
  `BattleSession::carrier_room`;
- then the ships either escape with their hangars or are destroyed with
  them.

Not exact:
- port: the Death Star takes no squadron, because the port's Death Star
  has no hangar.
- hyp: a squadron already inside a hangar (state 1) stays with its
  carrier. Which list it sits in is untraced. `FUN_005afd40` puts every
  object into the global registry (`DAT_006bc470` `+0x14`, through
  `FUN_005a64e0`), and the hangar setup's `FUN_005af4b0` adds it to the
  abstract manager's `+0x4c` and `+0x58`. If the side's `+0x64` held it,
  `FUN_005afe40` would refuse it and it would be destroyed. This hypothesis
  is narrower than the one it replaces.

## Tests

- `a_leaving_group_docks_on_its_home_carrier_first_and_needs_room`
- `a_ship_that_cannot_jump_takes_the_squadrons_inside_with_it`
- `fighters_leave_only_with_a_jumping_carrier` (kept, its `hyp:` removed)

Both new tests fail with the previous `leave`.

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2995 passed, 0 failed |
| `replay_manifest -- --ignored` (original data) | 2 passed; the golden is unchanged |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 7 tested: 7 caught |
| Release WASM SHA-256 | `bb87d8c35e7d0640eb5cbf26f8cd984d267be1f47df67b37c90d07b75c494e29` |
