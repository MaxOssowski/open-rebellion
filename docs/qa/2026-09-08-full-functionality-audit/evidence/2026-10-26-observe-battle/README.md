# Observe Battle hands the player's side to its commander (P24)

Date: 2026-10-26. Follows `../2026-10-26-computer-targets/`.

## The original

- Each side has a commander (`FUN_005be160`, `FUN_005c7cb0`). Its mode
  `+0x08` (`FUN_005c7e10`) is 1 when the computer commands that side.
  - In single player, `FUN_005be350` gives the non-local side mode 1 and
    the local side mode 0.
  - Switching to mode 1 clears the first-review flag `+0x14` and records
    whether a trench run is under way in `+0x18`.
- The Battle Options button `0xdd03` (`FUN_005dae10`) sends command `0x7b`
  (`FUN_0059fd40`), which flips the local commander between 0 and 1. This
  is the manual's "OBSERVE BATTLE / TAKE COMMAND" (manual line 5471).
- In mode 1 the commander's count `+0x40` falls by 1 each turn. It starts
  at 0, so the first review comes at once, and the rest every sixth turn.
  It reviews as the computer's does (`FUN_005c9200`).
- The withdrawal test skips the local side in mode 1 (`0x005c8a8f`).
- In mode 0 the commander only refreshes the interface.

## The port

- `BattleSession::player_observing` and `observer_review_countdown`
  (`+0x08`, `+0x40` of the player's commander).
- `set_player_observing` restarts the count when Observe is switched on.
  Pressing it again while already observing keeps the count.
- `ai_side_leaves` links the player's forces to targets on the observer's
  reviews (`link_commander_targets`, shared with the computer's review).
  It never withdraws the player's side.
- The Battle Options Observe toggle (tactical_view.rs) now sets it on the
  session as well as in the interface state.
- When the player takes command again, the reviews stop. The units keep
  their targets, as the original's forces keep their `+0x8c`.

## Tests

- `observing_hands_the_player_side_to_its_commander`:
  - no player links before Observe;
  - one link on the next turn's updates, with the count at 5;
  - pressing Observe again keeps the count;
  - Take Command stops the links;
  - switching Observe on again resets the count to 0.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2848 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T06-55-53-146Z-1260581`) |
| Replay equivalence | passed |
| `cargo mutants` | 19 tested: 16 caught, 2 unviable, 1 missed; with the repeated-press case, the targeted rerun caught all 3 on `set_player_observing` |
| Release WASM SHA-256 | `e42062b407572dd627af5a202e57b705d1882c525cf76f8237e5317f0e826338` |
