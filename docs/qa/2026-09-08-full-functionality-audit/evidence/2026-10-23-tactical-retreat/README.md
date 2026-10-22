# Retreat in the tactical battle (P24)

Date: 2026-10-23.

## The original

- One battle object with two side commanders serves both the abstract
  and the tactical mode (`FUN_00596f30`, `FUN_005af5f0`;
  `ghidra/notes/abstract-battle.md`).
- The AI player answers the battle choice with its retreat word
  (`FUN_00485d50`), whichever mode the player picks.
- A side with the order leaves under `FUN_005d19a0` (strength under 0.33
  of the enemy's, none left, or a stalemate mark). `FUN_005d14d0` step 5:
  each capital ship with a working hyperdrive escapes (`FUN_005af910`,
  state 9); the others are destroyed (`FUN_005af7a0`).

## Before

- Take Command never passed the AI side's order: the AI side never left a
  tactical battle.
- A ship that left the tactical field (`retreated`, no longer `alive`
  there) came back to the strategic map destroyed:
  `tactical_flow::apply_results` kept only `alive` ships.

## Change

- `BattleSession::retreat_allowed` / `retreat_done`. At the battle's start
  the app sets the AI side's order from `ai_strategy::retreat_orders`; the
  player's side stays unset.
- Each tactical step asks `abstract_battle::side_should_leave` for the AI
  side and runs the shared `abstract_battle::leave` (the abstract battle's
  step 5, moved out of `AbstractBattle::retreat`). The field counts a ship
  that escaped as gone. port: the jump is not drawn.
- `apply_results` keeps an escaped ship with its hull.

## Gates

- Workspace 2686 passed, 0 failed. Both new tests fail without their
  change: `a_ship_that_escaped_the_battle_keeps_its_hull` (the fleet was
  removed) and `the_ai_side_of_a_tactical_battle_leaves_under_the_same_rule`
  (with the call removed).
- Mutants on the touched core code: 18 tested, 16 caught, 1 unviable,
  1 missed: `||` -> `&&` in `ai_side_leaves`'s early return. It is
  equivalent: `side_should_leave` checks the order and `retreat_done`
  again through `can_leave`; the early return only spares a session clone
  each step.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `d29957926cd188cd5b0b19f849a646ac0b4476d3ae2bf880f4370587c4a000bc`.
