# The battle results' forces list has the original's scroll bar (P24)

Date: 2026-10-27. Before this stage, the forces list on the Battle Results
Alliance and Imperial pages scrolled only with the wheel, a whole row at a
time. It drew no bar and showed only the rows that fit in full. No window
in the port drew the original list scroll bar.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository; the call arguments were checked in its objdump),
and STRATEGY.DLL's resources read with a PE parser.

## The original

### The widget and its art

- `CoolDragList` (`FUN_00607ea0`) has no Win32 scroll bar (style
  `0x56010000`). Its scroll host (`+0x94`, vtable `0x66e0c0`) holds a
  "Scroll Bar" window.
- `FUN_0046f360` creates that window at `0x4704be` with
  `FUN_0060a490(.., list, 7, 0x299d, ..)`. It uses `FUN_0060aa70` and
  vtable `0x66e2e8`.
- The art is STRATEGY.DLL RCDATA 10653, read by `FUN_0060af70`:

| Field | Value | Use |
|---|---|---|
| [0] | 12 | width |
| [1] | 10656 | track tile |
| [2], [3] | 10685, 10686 | up arrow, normal and pressed |
| [4], [5] | 10687, 10688 | down arrow, normal and pressed |
| [6] | 10689 | thumb top cap |
| [7] | 10690 | thumb middle, tiled |
| [8] | 10691 | thumb bottom cap |
| [9] | 10684 | not read |

### Geometry

- `FUN_0060fa80`: `MoveWindow(bar, list.w - 12, 0, 12, list.h)`. The
  13-wide art is placed at `12/2 - 13/2 = 0`, so it is clipped to 12.
- The up arrow is at the top and the down arrow 9 above the bottom. The
  track lies between them.
- `FUN_0060fca0`: the thumb is `vis * L / total` high, at least 12. It
  sits `(L - t) * first / (total - vis)` down the track.
- `FUN_00609ae0`: the range is `content - view`. Content that fills the
  list gains 5 pixels.
- `FUN_0060b5d0`: the bar is hidden when the whole range is in view.

### Behaviour

- An arrow is a strobe button. Released over the arrow, it steps once by
  the list's `+0xc0`, which `FUN_0046f360` sets to 10. Held, it repeats
  (`FUN_0060f1e0(50, ..)`, threshold `0xfa / 50`).
- `FUN_0060fd50`: a press on the track pages by the list's height (166),
  and again every 150 ms (`SetTimer(.., 0x96)`).
- A press on the thumb drags it (`SetTimer(1000, 0x32)`).
- The list itself: `FUN_00607ea0(.., 0x25, 0x88, 0x15b, 0xa6, ..)` at
  `0x47041f`, that is (37, 136), 347 by 166, step 10.

## The port

- `crates/rebellion-render/src/list_scroll_bar.rs` is the widget, kept
  for the other lists to use. It holds the geometry, the art pieces and the
  press, hold and release logic (`update`); the caller draws the pieces.
- On the Battle Results forces pages (`tactical_view.rs`):
  - `battle_result_scroll` is now the first visible pixel, not a row;
  - the list clips its cells to its 347 by 166 window
    (`PageOp::Clip`, macroquad's scissor), so the top 26 pixels of a
    third 70-high row show, as in the original;
  - the bar is drawn from the STRATEGY.DLL art and takes the pointer;
  - port: the wheel moves one row, within the bar's range. The original
    has no wheel.
- The browser fixture `production-battle-results-presentation` now has
  five squadrons a side, so four stay operational and the Fighters list
  scrolls.
  - Its expected layout follows: 8 objects a side, extent 124
    (`FUN_005ab650`, 100 + 3 per object), and strategic fighters 5 -> 4.
  - Two probes were added:
    - `battle-results-scroll-bar`: the up arrow, down arrow and thumb
      top match their BMPs pixel for pixel at window (372, 136), (372, 293)
      and (372, 145);
    - `battle-results-scroll-step`: a click on the down arrow moves the
      thumb's top to 150. The range is 280 + 5 - 166 = 119, and the
      86-high thumb travels 62.

Captures (Linux Chromium, 640x480, run `2026-10-27T13-14-56-508Z-703491`):

| Capture | SHA-256 |
|---|---|
| alliance `results-category-2.png` (Fighters, bar shown) | `e1614e3b61224462671d7724c8b9d56d7ca1b35c1a16de37e6948a04b9919018` |
| alliance `results-fighters-scrolled.png` (after one arrow step) | `f80ef672ee80241c1b3ae459bae3e191f0242b59ef8f7fceacc36deef2b701ec` |
| empire `results-category-2.png` | `fba65ed2220dc06e0bd857fb2e076421e09a130e1c17ef51fc2d3bd8fbb03e63` |
| empire `results-fighters-scrolled.png` | `3881958a0ac90ef41674e3c3ac57bc488f314f80ecd66b30103978113b5cb9d6` |

The scrolled capture was inspected. The bar shows at the list's right
edge, and the rows have moved up 10 pixels, with the third row clipped at
the list's bottom.

Not exact:
- hyp: the arrow strobe's 50 ms period was read from `FUN_00601f40`'s
  arguments.
- port: the thumb drag follows the pointer every frame, not on a 50 ms
  timer.
- The other lists (the Battle Alert's, the Fleet Finder's, the Defenses
  and Missions windows') still draw no bar. They can now use the same
  widget.
- No A0 capture of the original's bar on this page exists. The art and
  geometry come from the static reading, and the pixel probes compare
  against the original BMPs.

## Tests

- `list_scroll_bar`: the six widget tests (placement, hiding, thumb,
  art, arrows, paging, drag).
- `the_forces_list_scrolls_by_pixels_under_its_bar` (replaces
  `the_wheel_scrolls_the_forces_list_a_row_at_a_time`, which encoded the
  port's whole-row scrolling).
- `the_forces_lists_bar_takes_the_pointer_in_window_coordinates`.
- `a_ships_picture_is_its_classs_portrait_and_two_rows_and_a_part_fit`
  now expects three pictures: the third row's top shows inside the
  166-high list (`FUN_00607ea0` at `0x47041f`).

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2993 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 310 tested: 296 caught, 8 unviable, 2 timeouts, 4 missed; a test added for one of them (`part_at`, 24 tested: 23 caught, 1 unviable) |
| Release WASM SHA-256 | `75eda0f66789c4a7fe911fb801560f05ada198909efcdc628d1404b883cc5372` |

The first browser run of the grown fixture failed in three ways:
- the fixture's own expectations (layout, participant and squadron
  counts), which were updated as above;
- the persistence counts (5 -> 4);
- one port fault. A press and its release in the same frame (a quick
  click) dropped the arrow's step. `list_scroll_bar::update` now handles
  both, and `an_arrow_steps_on_release_and_repeats_while_held` covers it.

Mutants:
- The two timeouts turn a tiling `+=` into `-=`, which makes its loop
  endless.
- Three of the remaining missed mutants replace a function that draws
  into, or reads the input of, a live macroquad window with `()`:
  `draw_battle_result_force_details`, `battle_result_list_input` and
  `draw_page_ops_at`. Their logic sits in tested pure functions
  (`battle_force_page`, `battle_result_bar_ops`,
  `battle_result_list_step`, `page_clip_on_screen`).
