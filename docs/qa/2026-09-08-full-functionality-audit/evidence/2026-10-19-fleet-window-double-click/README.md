# The Fleet window's double click and its gate (F-007, F-009)

Date: 2026-10-19.

The Fleet window gate (`tools/interface-parity/fleet-window.mjs`, 48
cases) failed intermittently. It failed on commit `a4a224de` in 3 of 4
runs, and on the unchanged `077a997d` in 1 of 1. It failed in two cases,
each with its own cause.

## `join-fleet`: a click on the menu, then on the list, counted as a double click

- The case chooses Move from a fleet's popup menu, then clicks the other
  fleet's entry. Sometimes the joined fleet was listed expanded (4 rows
  for 1 fleet of 3 ships).
- The left list expanded a fleet on `response.double_clicked()`. In egui
  0.31 that flag is global and timed alone (`max_double_click_delay`
  0.3 s, `input_state/mod.rs`), so the menu click and the entry click
  counted as one double click whenever they came within 0.3 s.
- Win32 posts `WM_LBUTTONDBLCLK` only when both clicks land in the same
  window, within the double-click time and inside the 4 by 4 rectangle
  (`SM_CXDOUBLECLK`) centred on the first. The popup menu is another
  window.
- Fix:
  - the open window keeps where and when the last primary click landed in
    its list (`OpenFleetWindow::last_click`);
  - a click elsewhere clears it;
  - a second click within the delay and 2 logical pixels of the first is
    the double click (`win32_double_click`).
- This follows the Fleet Finder's own list (`last_clicked`). It also keeps
  a real double click right after a click elsewhere, which egui takes for
  a triple click.
- This is a player-facing bug as well, not only a test artifact.

## `travel`: the fixture read the other side's arrival line

- The fixture's `last_regiment_message` took the last `Regiment ` line of
  the whole log.
- The AI plays the other side, and its regiments' arrivals go to its own
  index (`RailAudience::side`, Unit Arrival `0xd`, `main.rs`). So when an
  AI regiment arrived on the same day as the player's ("Svivren" against
  "F'tral"), the fixture reported the AI's line.
- Fix: the fixture reads only the lines the player's side receives
  (`RailAudience::includes`). The case's assertion is unchanged.

## Tests

- `fleet_window.rs`:
  `a_double_click_counts_only_when_both_clicks_land_close_in_the_list`.
  It covers:
  - a click elsewhere, then on the entry;
  - two clicks 3 pixels apart, across and down;
  - two clicks 2 pixels apart;
  - a click elsewhere, then a double click;
  - three clicks.
- `interface_test_fixture.rs`:
  `the_fleet_load_watch_reports_only_its_own_regiment_as_travelling` gains
  an Empire-only arrival line after the player's.
- Both fail without the change (`fail-without.txt`).
- `a_double_click_on_a_fleet_lists_its_ships_and_another_hides_them`
  still passes.

## Gates

- The Fleet window gate passed all 48 cases in 5 of 5 runs, against 1 of
  5 before (`fleet-window-runs.txt`). Interface WASM of the last run:
  `4032248400d80cafa8887ddd38aa177d6d67c8c09fe25793648c499179043d91`.
- `cargo test --workspace`: 2449 passed, 0 failed.
- Clippy on the two touched files: no warnings.
- The replay equivalence check passes with the unchanged golden
  (`v1:20ee82a2951ad9aa` -> `v1:aae0de7bbef402c1`) and 0 browser errors.
  Release WASM SHA-256
  `9da139533b0908586dc79fbccbde8632bd5de392e2b0062abea117d4b176764a`.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.6 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 5.0 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`fleet_window.rs`,
  `interface_test_fixture.rs`): 40 mutants, 36 caught, 2 unviable, 2
  missed.
  - `at.1 - y` with `/` is now caught by the 3-pixel vertical case.
  - `point.x - local.min.x` with `+` in `logical_point` is equivalent:
    both clicks take the same offset, and only their difference is
    compared.
- No simulation file changed.
- Screenshots stay under `target/` (original artwork).

## Still open

- The Sector window's planet icons (`sector_window.rs:788`) and the
  mission dialog's agents (`mission_dialog.rs:975`) still use egui's
  global `double_clicked()`.

Hashes of the committed files are in `SHA256SUMS`.
