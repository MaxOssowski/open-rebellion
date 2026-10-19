# Character command: the Command submenu and the Fleet Status rows (F-038)

Date: 2026-10-19.

`ghidra/notes/character-command.md`. This is stage 3 of the tracker item
"Dowodzenie i jego utrata przy ranie". The name prefix is held back for a
decision (below).

## The original

- A character's Command parent (`0x160`) opens None, Admiral, General and
  Commander. These are STRATEGY records `0x260..0x263`, read from
  `STRATEGY.DLL` for this stage:

| Order | Parent | Sort | TEXTSTRA | Bitmap (word 10) |
|---|---|---|---|---|
| `0x260` | 352 (`0x160`) | 402 | 12354 None | 11902 |
| `0x261` | 352 | 409 | 12356 Commander | 11902 |
| `0x262` | 352 | 406 | 12358 Admiral | 11902 |
| `0x263` | 352 | 408 | 12360 General | 11902 |

- Each order is checked by `FUN_004ed750` and its base check
  `FUN_004f9860`: the ordering side's, active and not destroyed.
- The Fleet Status window's rows 12358, 12360 and 12356 name the fleet's
  side's holder aboard (`FUN_004fd790`), else 34658 "Not Assigned".

## Port

- `object_menu.rs`:
  - the Command submenu, enabled per row;
  - the held kind checked with bitmap 11902, as `agent_menu.rs` does
    (`hyp:`);
  - Command as a submenu parent is enabled (`FUN_00442590`).
- `main.rs`:
  - `command_rows` (`order_allowed` per row);
  - `PanelAction::OrderCommand` (`give`), which logs
    `[interface] command=...`, or "Command refused".
- `character_command::order_allowed` adds `FUN_004f9860`'s side, active
  and destroyed tests. port: its en route test is not made.
- `status_rows.rs`: the fleet's Admiral, General and Commander.
- `interface_test_fixture.rs`:
  - scenario 68, `CharacterCommand`: the agent's class can command;
  - the object menu report gains the Command row and the submenu's
    command rows.
- **Held back: the name prefix** (`FUN_004f17f0`, a rename to TEXTSTRA
  class `+0x30` + `0x6800`/`0x6c00`/`0x7000`). The port keeps the
  character's name but not its TEXTSTRA id.
- No save format change.

## Tests

- `object_menu.rs`:
  - `a_characters_command_submenu_lists_none_admiral_general_commander`;
  - three tests that asserted the disabled Command parent now expect it
    enabled (`FUN_00442590`, the rule Planetary Bombardment already
    follows).
- `status_rows.rs`:
  `fleet_status_names_the_fleets_side_s_admiral_general_and_commander`. It
  fails with the old "Not Assigned" rows (`fail-without.txt`).
- `character_command.rs`: the base check cases (other side, inactive,
  destroyed).
- `interface_test_fixture.rs`: the scenario table and the object-menu
  reporters include scenario 68.

## Browser acceptance (`acceptance.json`)

Run with `agent-browser` on the interface build, fixture 68, both sides.
Each side right-clicked the agent (Mon Mothma) and opened Command, then
chose in turn:
1. Admiral;
2. Admiral again;
3. General.

- The log reads:
  - `command=0x262 ... now=Some(Admiral)`;
  - `command=0x262 ... now=Some(None)`;
  - `command=0x263 ... now=Some(General)`.
- No console errors.
- The submenu showed None, Admiral, General and Commander, with the
  bitmap on the held kind.
- Screenshots are not committed (original artwork):
  - `target/acceptance/character-command/{alliance,empire}-{1-submenu,2-admiral-checked,3-cleared,4-general-checked}.png`;
  - their SHA-256 are in `screenshots.sha256`.
- In fixture 44, Mon Mothma's class cannot command, and all three ranks
  were disabled (`submenu-before.png`).

## Gates

- `cargo test --workspace`: 2461 passed, 0 failed.
- The seed-42 golden is unchanged (`v1:4d8201e965d27089` ->
  `v1:2229fcf258823750`).
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `2d2f20a579a3b6f17537fe39ec7042dd8c8bcb177d40e0c1dc34dbffd70452a9`).
- The Fleet window gate passes all 48 cases (interface WASM
  `2b74c22974f78fdee1571a51da3600c5eb579b0701365c7505cdde7e31602495`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.5 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.8 ms |

- The first performance run had read 2.8 ms at the 99th percentile. A
  headless Chromium left over from the acceptance session was using
  8.9 cores; with it closed, the run above was repeated.
- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`, `rebellion-render`):
  26 mutants, 20 caught, 6 unviable.

Hashes of the committed files are in `SHA256SUMS`.
