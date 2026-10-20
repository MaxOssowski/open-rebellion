# Near Uprising warns the holder when a garrison's surplus falls to 0 (0x148)

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## The original

- `System_SetTroopRegSurplus` (`FUN_0050a230`) stores the troop surplus
  and, only on a change, runs the side views' sync. The sync calls the
  System's slot `+0x28c`, `SystemTroopRegSurplusNotif` (`FUN_00511fd0`),
  with the old and the new value. That posts notification `0x148`
  through `FUN_0053fcf0`.
- Near Uprising (`FUN_00498cb0`) answers `0x148` for the holder's side
  only. It speaks when all of these hold:
  - the system is Garrisoned (`+0x88` bit 8, `System_SetGarrisoned`
    from `FUN_0050ba90`);
  - it is not in revolt (bit 2);
  - the new surplus is 0 and the old one was not.
- Its texts are TEXTSTRA `0x7070` "{1:1} Near Uprising" and `0x7071`
  "Unrest has pushed {1:1} close to uprising.".
- A revolt starts when the surplus falls below 0 (`FUN_0050b800`). A
  surplus of exactly 0 is the last step before it.
- Argument order: a class's slot `+0x24` takes the new value first and
  the old second.
  - System Joins (`FUN_00499760`) on `0x100` requires its first value to
    be the system's current side.
  - Smuggling (`FUN_00499130`) on `0x145` reads its first value as the
    new percent (0 = smuggling ends). That is how `ffb0ea29` reads it.

## What the port did before

The economy computed the surplus every day and reported no change, and
nothing posted Near Uprising.

## Port

- `EconomyEvent::TroopSurplusChanged { system, previous, surplus }`, on
  a change only. The setup pass (`5771cdcb`) keeps the starting values
  silent. The headless integrator records no telemetry for it.
- `story_messages::near_uprising_message` is `FUN_00498cb0`. The app
  posts it on the Popular Support rail with the holder's background
  (`0x3f0` / `0x3f1`, `message_pictures::near_uprising`).
- port: "Garrisoned" is any regiment at the system.

## Tests

- New: `a_troop_surplus_change_is_reported_with_the_old_value`. It fails
  without the event (`fail-without.txt`).
- New: `near_uprising_warns_the_holder_when_a_garrisoned_system_s_surplus_falls_to_zero`.
- `cargo test --workspace`: 2497 passed, 0 failed.
- `cargo mutants --in-diff`: 14 mutants, 12 caught, 2 unviable.

## Gates (`f23`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `b691fdbbe5aa04144cf15fc52f9d59cc94360db81cecf5b617a4939d0d684e8c`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- `provenance-scan check`: 11 uncited items, unchanged.
- Not checked: the message in the browser. No fixture scenario brings a
  garrisoned system's surplus to 0.

Hashes of the committed files are in `SHA256SUMS`.
