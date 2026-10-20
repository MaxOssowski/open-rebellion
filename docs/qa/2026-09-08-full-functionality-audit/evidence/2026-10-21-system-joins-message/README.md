# A system the support moves joins a side or declares neutrality (0x100)

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## The original

- The holder hook `FUN_00510d70` (System slot `+0x208`) runs when the
  holder bits of the control word change.
  - It sets `+0x88` bit `0x2000` (LoyaltyCausedCurrentControlKind) when
    neither the old nor the new word has a regiment side (bits 4-5).
  - It then runs `FUN_004f6f40`, which hands the facilities over and
    posts `GameObjControlKindNotif` (`FUN_004fbbd0`, `0x100`).
- System Joins / Joins Enemy / Declares Neutrality (`FUN_00499760`)
  speaks while bit `0x2000` is set. Its first value must be the system's
  current side.
  - A system joining the viewer's side takes TEXTSTRA `0x7038` / `0x703b`.
  - A system joining the other side takes `0x7039` / `0x703c`.
  - Both name the new side (kind 1).
  - A system left with no side takes `0x703a` / `0x703d` "Divided loyalty
    on {1:1} has caused that world to abandon the {2:4} cause.", naming
    the side it left.

## What the port did before

- The app set the new holder and nothing else.
- The facilities did not follow the holder in the app. The headless
  integrator did hand them over, so the app and the simulation
  disagreed.
- No message was posted.

## Port

- `story_messages::control_change_texts` is `FUN_00499760`.
- On `ControlResolved` the app now:
  - hands the facilities over (`hand_over_facilities`, `FUN_004f6f40`);
  - posts the message for the player's side when no regiment is at the
    system.
- port: an empty planet stands for bit `0x2000`; the port keeps no
  regiment bits (`system-control.md`, hyp).
- The app's occupation (`tactical_flow::apply_system_occupation`) hands
  the facilities over too, as the integrator's does.

## Tests

- New: `a_system_moved_by_its_support_joins_a_side_or_declares_neutrality`.
- New: `an_occupation_hands_the_system_s_facilities_to_the_new_holder`.
  It fails without the hand-over (`fail-without.txt`).
- `cargo test --workspace`: 2501 passed, 0 failed.
- `cargo mutants --in-diff`: render 11 of 11 caught; `tactical_flow` 1
  of 1 caught.

## Gates (`f26`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged. The
  headless path already handed the facilities over.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `20c11bc8457984b6995c05cc4b935a1c519c7e319779baeaa57e7c901526e1ac`).
- The Fleet window suite passes 54 cases.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- Not checked: the message in the browser.

Hashes of the committed files are in `SHA256SUMS`.
