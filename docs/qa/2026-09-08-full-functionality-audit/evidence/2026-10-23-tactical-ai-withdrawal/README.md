# The AI side's tactical withdrawal (P24)

Date: 2026-10-23.

## Source

- The tactical battle has its own manager, `DAT_006bc4a0` (`FUN_005aaf90`,
  built by `FUN_005be0f0` -> `FUN_005be160`). It is not the abstract
  battle object `DAT_006bc4a8` (`FUN_005af5f0`).
- The manager's commanders:
  - `+0x08`: vtable `0x66c7a0`, side 0 (the Alliance), vector (0, 0, 1).
  - `+0x0c`: vtable `0x66c778`, side 1 (the Empire), vector (0, 0, -1).
  - The battle start `FUN_00599c90` gives them the retreat order
    (`FUN_005a8af0`, `+0x80`).
- Each commander steps through `FUN_005c8780` (`FUN_005a9d10`):
  - Every sixth call (`+0x40` from 6.0, down by 1.0) it re-plans
    (`FUN_005c9200`).
  - When the enemy's strength is above 0 and its own is under `+0x24`
    (0.33) of it, or 0, and it may leave (`FUN_005c7e80`), it runs
    `FUN_005ad530`. It does so for any side but a player-controlled one,
    and not while a trench run is under way.
  - That is the same withdrawal as the player's: 30 s (150 s for the
    Empire with an operational Death Star), order 3, the flight to the
    edge, and the jumps.
- The s42 port instead ran the abstract `leave` (`FUN_005d14d0` step 5)
  for the AI side. It removed the AI's units at once and destroyed those
  without a hyperdrive.

## Change

- `BattleSession::begin_withdrawal(side)` and `withdraw_fighters(side)`
  generalize the player's withdrawal (s46, s52, s54) to either side.
- `begin_original_fighter_recovery` moved from `rebellion-render` to
  `rebellion-core`, unchanged, so the AI's groups can be sent to a
  carrier.
- `ai_withdrawal_ms` holds the AI side's time. `advance_withdrawal` runs
  both sides.
- `ai_side_leaves` starts the AI side's withdrawal instead of
  `abstract_battle::leave`.
- Citation fixes:
  - The side vectors are cited to `FUN_005be160` (s54 cited
    `FUN_005af5f0`, whose abstract commanders hold the same values).
  - The "shared battle object" claim of s42 is corrected in the code and
    in `ghidra/notes/abstract-battle.md`.
- port:
  - the leave test is still the abstract `side_should_leave`, with the
    abstract strength, asked every step;
  - the trench-run check is not ported.

## Gates

- Workspace: 2716 passed, 0 failed.
  `the_ai_side_of_a_tactical_battle_leaves_under_the_same_rule` became
  `the_ai_side_of_a_tactical_battle_withdraws_like_the_player`, on this
  evidence.
- New tests:
  - `the_ai_sides_fighters_withdraw_by_their_hyperdrive`;
  - `a_recovery_reserves_one_free_place_on_a_living_carrier`.
- Mutants:
  - The first run, testing only `rebellion-core`, tested 69 and missed 19,
    all in the moved fighter code, whose tests are in `rebellion-render`.
  - Run again with both packages' tests and the new core test, only one
    remains: `fighter_capacity > 0` replaced by `>= 0` is equivalent,
    because a ship with no room is never chosen (`remaining > 0`).
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `3f4bc232e09d4eb72b2755e9b46a077ee30666206ac12cd4f12fd75071c2c484`
  (the tests changed after it; the code it builds did not).
