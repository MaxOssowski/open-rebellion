# The Battle Alert's list has its own scroll bar (P24)

Date: 2026-10-27. This follows `../2026-10-27-results-scroll-bar/`. The
Battle Alert's forces list scrolled only with the wheel, a row at a time.
It drew no bar and showed only the rows whose middle fit.

## The original

- `FUN_0044f860` creates the list at `0x44fd09`:
  - `FUN_00607ea0(.., 0x53, 0x28, 0x100, 0xea, ..)`: (83, 40), 256 by
    234;
  - its cells are 240 by 40 (`0x44fce2`, `0x44fce0`);
  - its step is 10 (`0x44fcd6`).
- It creates the bar at `0x44fd73`: `FUN_0060a490(.., list, 7, 0x29fc, ..)`.
- STRATEGY.DLL RCDATA 10748 (`0x29fc`) holds
  `(10, 10749, 10750, 10751, 10752, 10753, 10754, 10755, 10756, 10684)`:
  - width 10;
  - track 10749 (13 by 13);
  - up arrow 10750/10751 and down arrow 10752/10753 (13 by 9);
  - thumb caps 10754/10756 (13 by 6) and middle 10755 (13 by 12).
- Each piece is centred on the bar at `width / 2 - 13 / 2`: the arrows in
  `FUN_0060fa80`, the track in `FUN_0060fb80` and the thumb in
  `FUN_0060fca0`. On this 10-wide bar that is -1, so the art loses a
  column on the left and two on the right.

## The port

- `list_scroll_bar::BarArt` carries a bar's record:
  - `RESULTS_ART` (10653);
  - `ALERT_ART` (10748);
  - pieces at `width / 2 - 6`.
- `battle_alert_list` scrolls by pixels and clips its rows to the list.
- `alert_list_bar`, `alert_list_step` and `alert_pressed_arrow` give
  the alert list the same bar input as the results list.
- port: the wheel moves a row (40).
- The browser fixture `production-battle-alert-entry` has five
  squadrons a side, and its layout expectations follow:
  - 6 objects a side;
  - extent 118 (100 + 3 per object);
  - 12 participants.

## The evidence

Two probes were added:
- `battle-alert-scroll-bar`: the up arrow, the down arrow and the
  thumb's top cap match their BMPs pixel for pixel, at window (328, 40),
  (328, 265) and (328, 49), with the clipped columns ignored. That is 90,
  90 and 72 pixels.
- `battle-alert-scroll-step`: a click on the down arrow moves the thumb's
  top to window y 55. The probe scans for it.

Captures (Linux Chromium, 640x480, run `2026-10-27T19-44-19-223Z-3620105`):

| Capture | SHA-256 |
|---|---|
| alliance `battle-alert-tab-1.png` | `f0d7a0e9d6b6c189745d5eb36252b099a47371eba58c852d5a4462a45f6b0b25` |
| alliance `battle-alert-list-scrolled.png` | `c0aead98b9ba4698728465b4304dd2f0913608f3111ef03790dc29272ea15a33` |
| empire `battle-alert-tab-1.png` | `af7f3ebe8c5efe53d08df49c6860f12178087a1a9920550ce359c88efc4c2b40` |
| empire `battle-alert-list-scrolled.png` | `8af4f81d381f3d1f33cbb101aafcbeeefb0aff7faa143234fc6c1da6b24e14e9` |

The scrolled capture was inspected. The bar shows at the list's right
edge, and the bottom row is clipped at the list's edge.

## Tests

- `the_battle_alert_list_scrolls_by_pixels_under_its_bar` replaces
  `the_wheel_scrolls_the_battle_alert_list_a_row_at_a_time`, which
  encoded the port's whole rows.
- `the_wheel_scrolls_the_alert_list_only_over_it_and_a_new_tab_resets_it`
  is in pixels now, with a click on the down arrow.
- `the_battle_alert_list_rows_are_forty_high_from_83_40` sets the list's
  clip steps aside.

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 3005 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` (diff) | 155 tested: 142 caught, 8 unviable, 5 missed |
| Release WASM SHA-256 | `75eef40f32fd59d6a3f1c7e35e15d5d1abd88fafaf63580981ab3d08bf063642` |

Of the 5 missed mutants:
- Two replace `draw_battle_alert_text`'s returned row count; it draws into
  a live macroquad window.
- The three in `alert_pressed_arrow` and `battle_alert_list` are now
  caught by added cases: the arrow released or the pointer away, and the
  list scrolled 39, which still shows the first row's last pixel. A rerun
  over those two functions misses none.
