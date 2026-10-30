# The Status window, the Encyclopedia and the Message Index are modal (CMD-04-C016, C019, C020)

Date: 2026-10-30. Surface ledger CMD-04 (modality). Before this change:
- the Status window blocked the map only under its own rectangle, and
  the cockpit stayed live (C016 failed);
- the Encyclopedia blocked the map but not the cockpit;
- the Message Index blocked only its own rectangle.

## A0 (Windows 10 VM, Empire, days 218–219)

Captures are in `.artifacts/a0-vm/modal2/`; hashes are in
`a0-captures.sha256`. Each window was opened, then the test clicked the GID
control (537, 446), right-clicked the agent droid, clicked a rail button
and, for some windows, double-clicked a planet or clicked a cockpit
control. Each capture is compared with the one taken before the clicks:

| window | before | clicks | what changed |
|---|---|---|---|
| Manufacturing Status (Coruscant's Training Facility, through its object menu's Status) | `06-status.png` | `07`..`10` | only the pointer (y ≥ 462) |
| Galactic Encyclopedia (cockpit control 0x131) | `12-ency.png` | `13`..`16` | only the pointer, and the Topic box's blinking caret at x 268–269 |
| Message Index, list mode (rail) | `17-msg.png` | `18`..`21` | nothing |
| Message Index, message mode (`Uvena Joins Enemy`) | `23-msg-display.png` | `24`, `25` | nothing |

**Input was still live.** A click on the Index's own Fleet tab changed it
(`22-msg-tab.png`). Escape closed the Status window (`11-after-esc.png`)
and the Index (`26-closed.png`). No click made earlier took effect after
a window closed.

**Not tested in A0:**
- clicks on the windows behind a modal one, such as Coruscant's System
  window behind the Status window;
- the keyboard;
- the Battle Summary (C018).

## The port

`main.rs`: the Finders' modal gate is renamed `modal_window_open` and now
also holds while any of these is open:
- the Status window (`status_window_state.is_open()`);
- the Encyclopedia (`enc_state.open`);
- the Message Index (`message_index.is_some()`).

While any of them shows:
- the map takes no pointer;
- the strategic input is off (the GID control and menu, the speed menu,
  the droids' menus, the rail's Message Index command, the object menus);
- the cockpit's primary controls take no click.

## Browser (Arch Chromium 155, interface WASM `d8cb370b…`)

**`status-window.mjs`** passes all ten cases (`status-window-result.json`).
After the pixel comparison, each case clicks the GID control and checks
that:
- no GID menu opens;
- the window stays open.

In the two character cases the Status window's Encyclopedia button opens
the Encyclopedia, and the same check runs against it.

**`message-display.mjs`** passes for both sides
(`message-display-result.json`). With the index open, it clicks the GID
control and checks that no menu opens and the index stays.

**Run on the build without this change:**
- `status-window.mjs` failed all ten cases with "the GID control opened
  its menu under the Status window";
- `message-display.mjs` failed with "... under the Message Index".

The Encyclopedia check runs after the Status check, so on that build the
Status check failed first and the Encyclopedia check did not run on its
own.

**Still passing:** `fleet-finder.mjs` and `system-finder.mjs`.

**Failing before this change, and failing the same way or worse without
it:**
- `message-index-shell.mjs` differs by 470 pixels per case, both with and
  without this change.
- `encyclopedia-index-shell.mjs` differs by 62 pixels with this change,
  and by 107–398 without it.
- `encyclopedia-placement.mjs` waits for the main menu on the release
  site; since the movie flow change (1886c2be) the intro plays first.
- `encyclopedia-art.mjs` needs `REBELLION_EDATA_DIR`.

These are separate gaps.

## Tests

`cargo test --workspace`: all pass. The change is the app's input gating;
the browser cases cover it.
