# The second droid's Message Alerts menu (P34, CMD-07)

Date: 2026-10-30. Surface ledger cell CMD-07-C010 "message filter
submenu", which the reconciliation (562e61ad) left failing.

## The original

- **Opening.** A right-button release on an advisor panel runs
  `FUN_0042d050` with the panel's list. `0xdead` (the agent) builds the
  Agent menu (`FUN_0041d160`). `0xbeef` (the second droid: R2-D2 or SD-7)
  builds the Message Alerts menu (`FUN_0041d190` → `FUN_00436670` →
  `FUN_00487a60`).
- **The items** (`FUN_00487a60`, STRATEGY `RT_RCDATA` records, labels
  TEXTSTRA `0x3000` + command):
  - `0x11c` "Messages" (sort key 620);
  - the submenu `0x170` "Message Alerts" (630);
  - its children `0x171..0x179`, in the order Loyalty, Fleets, Mission,
    Resources, Manufacturing, Defense, Conflict, Chat (638), Advice
    (639).
  
  Each child is checked while its category alerts (`FUN_0048a1c0(mask)`
  bit 4 clear).
- **Choosing** (`FUN_00486fb0`):
  - `0x11c` opens the Message Index on All (`FUN_0041d770(1, 0x79)`);
  - `0x171..0x179` flip their category's bit 4
    (`FUN_0048a210(mask, option ^ 4)`), the option the Message Index's
    posting toggle flips too.

## The port

- **`agent_menu.rs`:**
  - `ALERT_RECORDS` hold the nine children;
  - `alerts_panel_contains` uses the second droid's aperture
    (`advisor::second_aperture`, `FUN_0042adb0`);
  - `open_alerts_menu_on_right_click` and `draw_alerts_menu` show
    "Messages" and the "Message Alerts" submenu (`game_menu`'s submenu
    with its 500 ms timer);
  - `alerts_command` maps the choice.
- **The app** flips the chosen category (`MessageLog::toggle_silent`,
  logged `[interface] message_alerts`), or opens the index on All.

## Browser (Arch Chromium, release WASM `cfc64044…`)

`agent-advice.mjs` now right-clicks the second droid. Both sides log
`[interface] message_alerts status=opened` and show the menu, with
"Messages" and "Message Alerts" and its submenu arrow (`alerts-menu`:
Alliance `32e5069d…`, Empire `86bd462a…`). Escape closes it. The run is
`…T05-43-48-776Z` (`agent-advice-result.json`).

Not compared in A0: the menu and its submenu. Choosing a submenu row is
covered by the unit tests, not the browser gate.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `the_alerts_submenu_lists_its_categories_in_sort_order_checked_while_they_alert`;
  - `the_alerts_menu_opens_the_index_or_toggles_the_chosen_category`;
  - `the_second_droids_panel_opens_the_alerts_menu`;
  - `a_right_click_on_the_second_droid_opens_the_menu_at_the_release`
    (two letterboxed screens);
  - `the_menus_first_row_opens_the_message_index`.
- `cargo mutants --in-place --in-diff` on `agent_menu.rs`: 40 mutants, 35
  caught, 4 unviable and 1 missed. The missed one turns
  `scale <= 0 || pointer over an area` into `&&`: a right click over a
  window that covers the droid. The Agent menu's opener has the same
  untested branch.
