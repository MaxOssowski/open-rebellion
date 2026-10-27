# Bombardment order, passenger injuries and the last cited hyps (P24)

Date: 2026-10-27. This is part of the review of the remaining `hyp:` tags.
The bombardment's and the Final Battle's tags are settled by static
reading of REBEXE.EXE. Two changes in behaviour follow: the targets'
order, and the injury step for characters aboard a ship lost to a
bombardment or a sabotage.

## The original

### The target order (`FUN_0058cba0`, `FUN_0053b760`)

- Each walk takes the system's child container (`+0x28`). It is a search
  tree, vtable `0x66a168`, built by `FUN_00584150`.
- Its comparator `FUN_00584320` orders by family (slot `+4`), then by
  object id (`+0x18`):
  `uVar3 < uVar4 || (uVar3 == uVar4 && param_1[6] < param_2[6])`.
- So the civilian targets come as shipyards (`0x28`), training centres
  (`0x29`) and construction yards (`0x2a`), then mines (`0x2c`) and
  refineries (`0x2d`). The regiments and the defense facilities follow the
  same rule.
- port: an object's place in its list stands for its id. The port
  sorts each list stably by family.

### The injury step for a lost ship's passengers

- A bombardment destroys a ship at full hull damage with reason `0xf`
  (`FUN_0058d540` at `0x58d657`: `push 0xf; ... call 0x501ae0`).
- A sabotage destroys it with reason 6 (`FUN_005746e0` at `0x57473b`).
- Both reasons are within 1..`0x10`, so `CapShip_vt11c` gives every
  character aboard the injury step (`FUN_004ef640` -> `FUN_0053ea00`)
  after setting them adrift.
- In a bombardment, those draws come in phase 2's per-ship pass, before
  phase 3's draws.

### Settled with no change

- A Death Star's destruction (`FUN_0058d990`) takes the system's objects
  with it:
  - `System_vtac` -> SetDestroyedReason (`FUN_004f71d0`) ->
    `GameObj_vt11c` (`FUN_004fb520`) hands each ship, facility and
    regiment its reason (`FUN_004f8710` -> `FUN_004f7ed0`);
  - so `FUN_0058dc10`'s battery walk finds none, and phase 3 is skipped
    (`+0x30`);
  - a fleet's `+0xb4` (`FUN_00524fb0`) keeps the fleet.
- The `+0x24 & 0x30` bits are a side view's copy index (`FUN_00540650`).
  They are 0 on the master objects a bombardment walks (`FUN_00540360`,
  `FUN_0053f660`).
- A new ship's packed `+0x64` word is 0: the CapShip constructor
  `FUN_005004e0` sets `[0x19]` to 0.
- The Final Battle moves Luke (`0x32000242`), Vader (`0x35000281`) and the
  Emperor (`0x34000280`). See `0x54b5bb`..`0x54b630` and `FUN_005434b0`,
  `FUN_00543760`, `FUN_00543800`.

## The port

- `bombardment::targets` and `civilian_facilities` sort by family.
- `bombardment::resolve` rolls a destroyed ship's passengers' injuries
  (`missions::passengers`, `roll_passenger_injury`) from its draws, at the
  ship's loss. `apply` then applies them after `destroy_ship` has set the
  characters adrift.
- `missions::roll_members` adds `CharacterInjured` and `DiedOfInjury` for
  the passengers of a sabotaged capital ship or Death Star, drawn from
  the mission's stream.
- `injure_passengers` now uses the shared `roll_passenger_injury`.

## Tests

- `the_targets_come_in_family_order` (new).
- `a_ship_the_batteries_destroy_injures_its_passengers` (new). It fails
  with no passengers found.
- `a_sabotaged_ship_rolls_its_passengers_injuries` (new). It fails with no
  passengers found.

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 3005 passed, 0 failed |
| `replay_manifest -- --ignored` (original data) | 2 passed; the golden is unchanged |
| `provenance-scan check` and its tests | 0 uncited items; 21 passed |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 46 tested: 43 caught, 3 unviable, 0 missed |
| Release WASM SHA-256 | `0c7b9ba8a6ed257c01bc6c7bd997185f3904390e75a074a0218233cd6ed25180` |

The first gate run found three things:
- the provenance scan's self-test flagged the literal 100 in
  `DrawFn::chance`, which became the cited `PERCENT_DRAW_SPAN`
  (`DAT_00661a88`);
- two mutants of `chance` survived, so a veteran of combat 91 (chance 9,
  draw 9) was added to the bombardment test;
- three mutants of the capital-ship arm survived, so
  `a_sabotaged_ships_passengers_are_those_of_the_ship_it_destroys` was
  added.
