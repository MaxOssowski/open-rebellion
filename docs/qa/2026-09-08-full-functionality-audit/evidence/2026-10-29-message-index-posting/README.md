# The Message Index's posting toggle and Select All (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy" (the Message Index around
the advice). This continues `../2026-10-29-message-index-rows/`. Its
"Post Messages" point was a misreading: the heading's striped button in
A0 is Select All (`0x90`), and the posting toggle (`0x66`) sits on the
right rail.

## The original

- **The buttons.** `FUN_004665f0` builds them:
  - Select All, `0x90`, at (`0x11a`, `0x57`), 56 by 20, art
    `0x2a94` / `0x2a95`, beside Delete (`0x91`);
  - the posting toggle, `0x66`, on the rail at (`0x1a7`, `0x93`) for the
    Alliance and (`0x1aa`, `0x94`) for the Empire, labelled TEXTSTRA
    `0x8005` "Post Messages Silently".
- **Select All** (`FUN_00467f10` case `0x90`) sets the selection bit
  (`+0x3c` bit 0) on every shown item.
- **The posting toggle** (`FUN_00467f10` case `0x66`):
  - **On a category** it flips bit 4 of that category's option
    (`FUN_0041d250` / `FUN_0041d230` → `FUN_0048a1c0` /
    `FUN_0048a210`, a node per category mask).
  - **On All Messages** it flips the default (`FUN_0041d270` /
    `FUN_0041d290` → `FUN_0048a1f0`) and clears every category's own
    value.
  - **A category without its own value** reads the default.
  - **The art** follows the bit:
    - while alerting, `0x2a78` / `0x2a7a`, labelled "Post Messages
      Silently";
    - while silent, `0x2ac7` / `0x2ac9`, labelled "Post Messages with
      Alert";
    - pressed, `0x2a79` / `0x2a7b`.

    `FUN_004697b0` sets it again on each category switch.
- **The bit's effect** (`FUN_0048a060`): a delivered message calls the
  advisor's event slot only while its category's bit 4 is clear. So a
  silent category still files its messages and lights its rail, but the
  droids do not comment.

## The port

- `MessageLog`:
  - holds the posting options (a default and per-category values);
  - its `posting_option` and `toggle_silent` follow the rules above;
  - `push` gives the advisor no event for a silent category.
- `message_index`:
  - `window_button_specs` gains `0x66` (its art by `posting_silent`)
    and, in the list mode, `0x90`;
  - a click on `0x66` returns `ToggleSilent`, which the app applies and
    logs (`[interface] message_index posting`);
  - a click on `0x90` selects every shown row (`select_all`).
- `bmp_cache` keys out the blue mat of:
  - the toggle's art (10872..10875, 10951..10953);
  - the Empire's Display Message button (10378 / 10379). Its blue
    corners showed before; A0 shows none.

## Browser (Arch Chromium, release WASM `18d159ef…`)

- `agent-advice.mjs` now also clicks Select All and the toggle twice. It
  passed for both sides at `…T22-44-23-318Z` (`agent-advice-result.json`,
  `posting.log`). Each side logs `mask=Some(512) silent=true`, then
  `silent=false`.

  | Side | select-all | posting-silent |
  |---|---|---|
  | Alliance | `bc364f8d…` | `d8239311…` |
  | Empire | `6ab71b3f…` | `1cb48d62…` |

  Select All shows every row's bar. The silent toggle shows `0x2ac7`'s R2
  with "...".
- `agent-objectives.mjs` passed at `…T22-44-46-008Z`.

## Open

- **Not drawn yet:**
  - the window's title ("Message Index", A0 at the top left);
  - the chosen category control's pressed look;
  - the rail's Display (`0x67`) and Compose Chat (`0x68`).
- **Not saved:** the port saves no messages and no posting options. A
  loaded game starts with every category alerting.
- **The menu items `0x171..0x179`** (`FUN_00487a60`), which check each
  alerting category, are not traced to a visible menu.
- **Not compared in A0:** the toggle's art and its effect on the droids.
  The capture shows the toggle only in its alerting state.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `a_silent_category_keeps_its_messages_from_the_advisor`;
  - `the_posting_toggle_shows_its_state_and_reads_the_categorys_option`;
  - `select_all_chooses_every_row_the_list_shows`;
  - `the_toggle_and_select_all_answer_their_clicks`;
  - `the_toggle_reads_the_shown_categorys_silence`;
  - `message_index_rail_buttons_key_out_their_blue`.
- Extended for `FUN_004665f0`'s `0x66` and `0x90`:
  `close_and_display_sit_on_the_rail_and_delete_shows_in_the_list_only`.
- `cargo mutants --in-place --in-diff`:
  - the first run (excluding the app's `main.rs`) found 29 mutants and
    missed 8 in the window's button handling;
  - after the new tests, `message_index.rs` alone gives 18 mutants: 16
    caught, 2 unviable.
