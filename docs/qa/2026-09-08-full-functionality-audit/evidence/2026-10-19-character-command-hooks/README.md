# Character command: the hooks that take it (F-038, stage 6)

Date: 2026-10-19.

`ghidra/notes/character-command.md`. This is stage 6 of the tracker item
"Dowodzenie i jego utrata przy ranie".

## The original

`FUN_004eeed0` (read from the image, `0x004eeed0..0x004eef64`) keeps a
command only while the character is:
- uninjured;
- not captured;
- existing (`+0x50` bit 6);
- not in a mission (container `+0x68` of family `0x50..0x7f`).

Four character hooks run it:

| Slot | Function | On |
|---|---|---|
| `+0x32c` | `FUN_004f1720` | injury |
| `+0x34c` | `FUN_004f1a80` | capture |
| `+0x11c` | `FUN_004f1340` | destruction (`FUN_004f7560` sets `+0x50` bit `0x8`, then calls the slot) |
| `+0x268` | `FUN_004f1490` | mission change (the base `Role_vt268` sets the role's parent at mission completion) |

## Port

- `character_command::keeps` adds the mission test, reading the port's
  `on_mission`, `on_hidden_mission` and `on_mandatory_mission` flags. A
  character on a mission loses its command at the day pass
  (`release_unfit`).
- `Character::mark_killed` clears the command.
- Notes corrected:
  - `character-injury.md` no longer says the command clearing is unported;
  - `character-command.md` names the two hooks and the Emperor's
    `FUN_005617b0` (seat of power, already in `seat_of_power.rs`).

## Tests

- `character_command.rs`: `a_death_or_a_mission_takes_the_command`. It
  fails without the change (`fail-without.txt`).

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all
24 runs are unchanged event for event against stage 5.

## Gates

- `cargo test --workspace`: 2471 passed, 0 failed.
- The seed-42 golden is unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `0d29263cd5e2b0b58eb6653360a2e430cf8f930bb384f17a797c87d1efc8f69a`).
- The Fleet window gate passes all 48 cases (interface WASM
  `865fce8c733ddcc403384e778b0ab6584cf13cbe135bb3899c5489273c998708`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.3 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 5.1 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`): 12 mutants, 12 caught.
- `provenance-scan check`: 85 uncited items, unchanged.

## Left in the item

- `FUN_004f9860`'s en route test (`+0x50` bit 4) in the order's check is
  not made.
- The Command submenu's checked row is a `hyp:` (the order's `+0x14`
  check is not read).

Hashes of the committed files are in `SHA256SUMS`.
