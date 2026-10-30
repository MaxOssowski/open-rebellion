# The Message Index's Display goes to a message's system (P34, CMD-08)

Date: 2026-10-30. Surface ledger cell CMD-08-C014 "navigate to object",
which `2026-10-06-surface-ledger.json` had failing: Display (`0x67`) was
never enabled.

## The original

- **When Display is enabled** (`FUN_004697b0`): the chosen message's
  kind (vtable `+0x10`) is 3, or 5 with a target (`FUN_004ece60`). Then
  Display shows `0x2916` / `0x2917` (Alliance) or `0x2918` / `0x2919`
  (Empire), labelled `0x8030` "Display". Kind 4 shows Encyclopedia
  (`0x2d20` / `0x2d47`). Otherwise it is disabled (`0x2acf` / `0x2ad0`).
- **What a click does** (`FUN_00467f10` case `0x67`):
  1. it takes the first chosen row in mode 1, or the viewed message in
     mode 2;
  2. for kind 3 or 5 it goes to the message's object `+0x5c`
     (`FUN_00429440`, which opens the object's window and chooses it, as
     a Finder's double click does);
  3. for kind 4 it opens the Encyclopedia on `+0x60`;
  4. then it posts Close (`0x28`).

## The port

- **Enabling.** `display_target` takes the chosen (or viewed) message's
  system. A message with a system stands for kinds 3 and 5 with a target.
- **The buttons.** `window_button_specs` adds Display in its enabled art
  when there is a target; `disabled_rail_controls` keeps its disabled art
  otherwise.
- **The click** returns `MessageIndexAction::Display(system)` and
  `Close`. The app opens the system's sector window, chooses the system
  and logs `[interface] message_index display`.
- **Fixture.** The EventMessages fixture's "Fleet Arrives" now names its
  system (`FUN_004981c0` sets the arrival's object).
- **port:** a message's object that is not a system (a fleet or a
  character) is taken as its system.
- **Not carried:** kind 4's Encyclopedia target.

## Browser (Arch Chromium, interface WASM `2d325170…`)

The new gate `tools/interface-parity/message-display.mjs` (fixture 57)
passed for both sides (`message-display-result.json`). It opens the
Mission rail, then checks three things:
- "Research Complete" (no system) leaves Display disabled, and a click
  does nothing;
- "Fleet Arrives" enables Display;
- the click closes the index and opens Akrit'tar's sector window.

| Side | display-disabled | display-enabled | after-display |
|---|---|---|---|
| Alliance | `ad2492fd…` | `3eaccb5d…` | `bc23b1a0…` |
| Empire | `63b0d812…` | `db25e353…` | `157854a6…` |

Not compared in A0: the enabled Display art and the window it opens.

## Tests

- `cargo test --workspace`: all pass.
- New test: `display_goes_to_the_chosen_messages_system_and_closes`.
- `cargo mutants --in-place --in-diff` on `message_index.rs`: 47
  mutants, 45 caught, 2 unviable.
