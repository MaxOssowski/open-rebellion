# Character command: the field and the orders (F-038, save v85)

Date: 2026-10-19. The user approved save v85 for this stage.

`ghidra/notes/character-command.md`. This is the second of four stages of
the tracker item "Dowodzenie i jego utrata przy ranie".

## The original

- Character short `+0x96` is `CommandKind`: 1 Commander, 2 Admiral,
  3 General (`FUN_004ee350`).
- Orders `0x260..0x263` set it (`FUN_0053ebb0`).
- `FUN_004ed750` refuses the order when the character:
  - is on a mission;
  - is injured;
  - is captured;
  - has a class that lacks the kind;
  - is ordered None while holding no command.
- `FUN_00561bb0` takes the kind from the side's holder in the same fleet,
  or on the same planet. Giving the character its own kind again clears
  it (`FUN_004ef400`).
- The injury and capture hooks clear the command (`FUN_004eeed0`).

## Port

- `rebellion-core/src/character_command.rs`:
  - `CommandKind`;
  - `order_allowed`;
  - `give`;
  - `keeps`;
  - `release_unfit`.
- `Character::command` (save v85).
- `Character::injure` clears it.
- Both day loops (`simulation.rs`, `main.rs`) call `release_unfit` after
  healing, for captures.
- port:
  - the base check `FUN_004f9860` is read as "alive";
  - `FUN_004eeed0`'s bit and key tests are a `hyp:`;
  - a capture clears the command at the day's pass, not in its hook;
  - `Damaged` is not kept;
  - `FUN_00560d50` is not ported.
- Nobody gets a command yet. The Command submenu and its handler are
  stage 3.

## Tests

- `character_command.rs`:
  - `the_command_orders_name_none_commander_admiral_and_general`;
  - `a_command_order_needs_a_free_uninjured_character_its_class_allows`;
  - `a_fleet_or_a_planet_has_one_holder_of_each_command`;
  - `an_injury_or_a_capture_takes_the_command`.
- Those functions are new, so their tests cannot pass without them.
- Without the clearing in `Character::injure`, the injury test fails
  (`fail-without.txt`).

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all
24 runs are unchanged event for event against stage 7 of F-036, since no
one holds a command.

## Gates

- `cargo test --workspace`: 2459 passed, 0 failed.
- Seed-42 golden regenerated for this cause (save v85: the new field is in
  the state):
  - initial `v1:20ee82a2951ad9aa` -> `v1:4d8201e965d27089`;
  - final `v1:1ddf726cecfd8477` -> `v1:2229fcf258823750`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `b8297366a18efbf465754257193c3846a8770c42858eb951a5f625f5b25bb8b1`.
- The Fleet window gate passes all 48 cases (interface WASM
  `053abc1faa57c6f19fa13d7148ae35e9cff79dc9e46210db546002cc659f4572`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.9 ms | 23.2 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.9 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - `rebellion-core`: 43 mutants, 42 caught, 1 unviable;
  - `simulation.rs`: 2 mutants, 1 caught, 1 unviable.
- Clippy on `character_command.rs`: no warnings.
- `provenance-scan check`: 85 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
