# The Agent menu's Galaxy Overview (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy", the Agent menu. The
reading is in `ghidra/notes/status-window.md`, "Galaxy Overview".

## The original

Galaxy Overview (`0x113`, Alt+O) opens the Status window on the galaxy
(`0xf1000004`).

- **Collecting.** `FUN_00443130` walks fifteen object lists of the viewer's
  side by family range. For each range, `FUN_0044a7a0` groups the existing
  objects by class: their count, and their summed maintenance for families
  `0x10..0x3f`.
- **Rows.** `FUN_0044aa20` makes one row per group:
  - the class's GOKRES list mini, 61 by 25;
  - the text `%03d`, two spaces (`DAT_006a874c`), `%04d`;
  - a key equal to the family, so the list sorts by family.
- **Layout.** The list's own format, `+0xe4 = 9`, centres the text across
  the 208-wide cell, from its top. The mini sits at the cell's corner.

## A0 (original game, Windows 10 VM, Empire)

The captures stay outside Git; their hashes are in `a0-captures.sha256`.

- **Day 139:** the top of the list.
- **Day 141:** the list again, then six captures scrolled to its end.

The rows, in family order:
- three regiment classes;
- four capital ship classes;
- TIE fighters (`020  0060`);
- two defense classes;
- two yard classes;
- mines (`014  0000`) and refineries (`013  0000`);
- one special force class (`001  0001`).

## The port

- `status_rows.rs`:
  - `StatusObject::GalaxyOverview` and `overview_rows`;
  - `IconRowLayout::Overview`;
  - each row's bitmap source (`StatusIconRow::source`).
- port: ship and fighter classes have no family byte in their `DatId`, so
  the rows take the families from CAPSHPSD (`0x14`, the Death Star `0x18`)
  and FIGHTSD (`0x1c`).
- hyp: rows of one family keep their mini's order (the keyed set of class
  resources). A0 does not contradict it.
- `status_window.rs`: the Overview layout. The mini is at (1, 0) of the
  cell; the text is centred across the 208-wide cell less 3, from its top.
- `agent_menu.rs`: Galaxy Overview is always enabled (`FUN_00487900`).
- `main.rs`: the menu's Galaxy Overview and Alt+O open the window.

## Browser acceptance (`tools/interface-parity/agent-objectives.mjs`)

Run `agent-objectives-2026-10-29T10-20-13-473Z` passed for both sides. The
WASM hash is `6fee94fe…` (`browser-result.json`). The test now also opens
Galaxy Overview, first from the menu's fourth row (`source=menu`), then
with Alt+O (`source=accelerator`).

No console errors appeared, and every request returned below 400.

Pictures: `{alliance,empire}-galaxy-overview.png`, a new game:
- Empire: regiments, ships, TIE fighters;
- Alliance: regiments, ships, X-wings, the headquarters.

Compared with A0:
- The mini positions, the text format and position, the name and the
  frame match.
- The counts differ because the games differ.
- port: no scroll bar.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `the_galaxy_overview_groups_the_sides_objects_by_class_in_family_order`;
  - `the_galaxy_overview_window_names_the_galaxy_and_lays_out_minis`;
  - `the_overview_rows_put_the_mini_at_the_cells_corner_and_centre_the_text`.
- Before the fix, the first browser run showed fighters and ships sorted
  before regiments (classes without a family). The family constants fixed
  that.
- `cargo mutants --in-place --in-diff`:
  - 65 mutants, 2 missed;
  - tests were added for both, a facility of each side and the galaxy
    background;
  - on rerun, `overview_rows` and `status_view` had none missed.
