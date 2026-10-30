# The Planetary System Finder (OBJ-02-C001..C003, CMD-10-C012)

Date: 2026-10-30. Surface ledger OBJ-02 (System Finder, System window and
System Defenses) and CMD-10 (primary strategic command controls). Before
this stage the cockpit's control 0x12d and F2 only logged
`status=pending_original_window`.

## The original

The trace is in `ghidra/notes/system-finder.md`. In short:

- **Class and opening.** Class `FUN_00460090`, vtable `0x0065a000`, window
  type 0x14, 470 by 330. F2 and the cockpit's 0x12d open it through
  `FUN_0042a000`.
- **Layout** (`FUN_00460270`): five side tabs; panel 10522; rail
  10584/10588; Close and Display.
- **List filters** (`FUN_004611d0`): side bits plus the explored flag.
- **Choosing** (`FUN_00609650`): a tab change runs the prefix choice
  through command 100. With an empty name box that is the first row. A
  typed name chooses the row it best begins.
- **Opening a system:** Display, Enter or a double click opens the
  system's sector window (`FUN_00429ce0`) and closes the Finder.
- **Escape** closes it (`FUN_00461590`).

## A0 (Windows 10 VM, Empire, days 217–218)

Captures are in `.artifacts/a0-vm/system-finder/`; hashes are in
`a0-captures.sha256`:

- `01-f2.png`: F2 opens All Systems with nothing chosen.
- `02-alliance.png` … `05-unexplored.png`: each side tab, showing its first
  row chosen.
- `06-all.png`: back to All.
- `08-enter.png`: "cor" and Enter open the Corellian sector beside the
  open Sesswenna window.

`07-typed.png` has the same hash as `08-enter.png`: it was taken after
Enter. So no A0 frame shows the typed state on its own.

## The port

- **`system_finder.rs`:** `SystemFinderState`, `SystemTab`, the sorted
  rows, `prefix_choice` and `draw_system_finder`.
- **`main.rs`:**
  - F2 and the cockpit control open it;
  - it joins the Finders' modal gate;
  - Display, Enter or a double click opens the sector window;
  - the global Escape chain leaves Escape to its own key slot, as for the
    other Finders.
- **Changes shared by all four Finders** (Fleet, Troop, Personnel and
  System):
  - **The panel lies under the base 10335/10336**, which is keyed on blue,
    so the base's rounded corners cover the panel's. The A0 captures of
    every Finder show those corners. The port drew the panel over them,
    so 86 corner pixels differed.
  - **The window shows at once.** egui's fade-in had made the first frame
    one step darker.
  - **The name box's caret is clipped to the box.** It had run one row
    past the box and blinked over the frame, which made `fleet-finder.mjs`
    fail at random.
  - **Escape is read before the name box**, which would otherwise take it.
- **`bmp_cache.rs`:** the Neutral and Unexplored tab images (10508/10509,
  10511/10512) key out blue. A0 shows the panel's white under their
  first pixel.
- **Fixture builds skip the intro.** Since the movie-flow change, every
  fixture page requested `data/video/000.webm`, which the fixture site does
  not carry. The 404 failed the gates' clean-console check.

## Comparison (Empire, A0 against the port, window at (125, 52))

These are pixel comparisons of the 470 × 330 window. They leave out the
title, the name label, the name box, the tab heading and the list. Text
runs about 9% wider without GDI hinting (accepted 2026-10-30), and the
list follows each game's state. 78,866 pixels are checked per tab:

| A0 | port | differing |
|---|---|---|
| `01-f2` All | `all.png` | 0 |
| `02-alliance` Rebel | `rebel.png` | 179, all under A0's pointer on the clicked tab |
| `03-empire` Imperial | `imperial.png` | 171, the pointer |
| `04-neutral` Neutral | `neutral.png` | 175, the pointer |
| `05-unexplored` Unexplored | `unexplored.png` | 171, the pointer |

The All tab's first eight rows read the same in both (Adega, Agrilat,
Akrit'tar, Algarian, Alk'lellish III, Allyuen, Anchoron, Annaj): the
galaxy holds the same systems in the same order.

## Browser (Arch Chromium 155, interface WASM `6c5973f1…`)

`system-finder.mjs` passes for both sides (`system-finder-result.json`).
For each side it checks:
- F2 opens All, sorted, with nothing chosen;
- each side tab chooses its first row;
- a typed name chooses the row it best begins, and Enter opens that
  system's sector and closes the Finder;
- the cockpit control reopens it;
- the GID control does nothing under it;
- Escape closes it.

Before the Escape fix, the last step failed. Before the intro fix, both
sides failed on the 404.

`fleet-finder.mjs`, which also covers the Troop and Personnel Finders,
passes every case (`fleet-finder-result.json`). Its expected image now
lays the panel under the keyed base, as A0 does.

## Tests

- New tests:
  - `the_window_and_its_controls_lie_where_the_original_places_them`;
  - `the_list_scrolls_as_little_as_it_must_to_show_a_row`;
  - `the_system_finders_neutral_and_unexplored_tabs_key_out_blue`, which
    failed without the key change;
  - the module's earlier five (filters, tabs, opening and tab choice,
    Escape with the box focused, prefix choice).
- `cargo test --workspace`: all pass.
- `cargo mutants --in-place` on `system_finder.rs` without `draw_system_finder`
  (covered by the browser gate): 164 mutants, 156 caught, 6 unviable,
  2 missed. Both missed mutants are equivalent: `<` to `<=` and `>` to `>=`
  in `scrolled_to`'s bounds, where the equal case returns the same scroll.
