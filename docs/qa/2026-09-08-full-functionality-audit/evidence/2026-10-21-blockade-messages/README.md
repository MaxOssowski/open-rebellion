# The blockade and evacuation messages take the original classes

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty": the blockade's
timers and troop withdrawal belong to this area).

## The original

- Blockade (`FUN_004960f0`) answers `SystemBlockadeNotif` (`0x14e`) only
  when the blockade bit is set. The end of a blockade posts nothing.
  - It needs a held system and a fleet there; it takes the first,
    `FUN_004ffdd0`.
  - The holder's side reads TEXTSTRA `0x70a2` "{1:1} Under Blockade" and
    `0x70a3` "{1:4} ships have been detected at {2:1}.  The world is
    under enemy blockade.".
  - Its `{1:4}` is the other side: `FUN_004f3dd0(viewer, 2)` for an
    Alliance holder and `(viewer, 1)` for an Empire one (`0x4961a3`,
    `0x4961c9`).
  - The other side reads `0x70a0` "Fleet Initiates Blockade of {1:1}" and
    `0x70a1` "{3:1} has initiated a blockade of {2:1}.", naming the fleet.
  - It is silent while the system's `+0x88` bit `0x4000` is set.
- Evacuation Losses (`FUN_00495e90`) answers
  `TroopRegDestroyedRunningBlockade` (`0x340`): `0x70a8`, `0x70a9` "The
  following units were lost running an enemy blockade of {2:1}.", and a
  `0x70aa` line for the regiment.
- The same reading confirms the Uprising class (`0x4994b6`, `0x4994dd`):
  there `FUN_004f3dd0` gives the holder itself, as `38ffc9a3` ported.

## What the port did before

- It posted "Blockade established at X", "Blockade lifted at X" and
  "Troops destroyed by blockade at X" to both sides.

## Port

- `story_messages::blockade_texts` and `evacuation_losses_texts`.
- A blockade's start posts the player's own variant, naming the first
  fleet at the system. There is no message without a fleet.
- Its end keeps the port's line in the log only, off the Message Index.
- A regiment lost running a blockade is named before it is removed, and
  reported to its side.
- port: the port keeps no `+0x88` bit `0x4000`.

## Tests

- New: `a_blockade_reads_differently_for_the_holder_and_the_blockader`.
  It covers both viewers, both holders, a neutral system and the
  evacuation texts.
- `cargo test --workspace`: 2511 passed, 0 failed.
- `cargo mutants --in-diff` (render): 12 mutants, 12 caught.

## Gates (`f40`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `f9b1505cc57c3328968f63aa4c02630407826d9e7790853698571554a3aed024`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- Not checked: the messages in the browser.

Hashes of the committed files are in `SHA256SUMS`.
