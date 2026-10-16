# Planetary Bombardment sub-menu: module 8, stage 4f (F-032, F-036)

Date: 2026-10-16.

The player now orders a bombardment the way the original does: from the
object pop-up menu's Planetary Bombardment sub-menu on a fleet or a sector
window's fleet icon. The reading is in `ghidra/notes/object-popup-menu.md`
("Submenus") and `ghidra/notes/bombardment-order.md`.

## Port

- `crates/rebellion-render/src/game_menu.rs`
  (`draw_game_menu_with_submenu`, `GameMenuState`):
  - a parent row is always enabled (`FUN_00442590`, flags 8);
  - the child window opens beside the parent window (`FUN_004ab3a0`): x =
    parent.x - width + 3, or parent.x + parent.width - 3 when that is
    below 0; y = parent.y + row.top - 3 when the child fits above the
    parent's bottom, else parent.y + row.bottom - height + 3;
  - arrows: STRATEGY 10117/10118 (side 1) or 10128/10129, at the row's
    left edge + 6;
  - `FUN_004aab50`:
    - the window's 500 ms timer, started when it is shown, opens the
      highlighted parent's submenu on its next tick;
    - `PtInRect` on the enabled rows picks the highlight, and a new
      highlight hides the submenu;
    - a click on the parent issues nothing;
    - Escape leaves the submenu first, then closes the menu.
- `object_menu.rs`: a fleet (`FUN_004ff8e0`) and a fleet icon
  (`FUN_00507290`) list `0x220..0x223` (STRATEGY sort 202..208, TEXTSTRA
  12314..12317). Each order is enabled by its `+0x18`
  (`bombardment::order_enabled`, which `ai.rs`'s validation of
  `AIAction::Bombard` now shares).
- `main.rs`: a chosen order becomes `PanelAction::OrderBombardment`. Its
  team is the fleet, or the side's fleets at the icon's system
  (`bombard_team`); its target is their system, with no confirmation
  (`FUN_00487cc0`). The order is validated and carried out at once
  (`FUN_00487740`, `BombardEnv`, `log_bombardment`, `destroy_planet`); a
  refusal is reported (`FUN_00487c90`).

port:
- The timer counts on egui's clock from the menu's first frame.
- The Command submenu (`0x160`) is not ported, because a character's orders
  are not ported.
- The older Bombard panel's button still orders General Bombardment.

No simulation change: save v55, the golden and the campaign are as in
stage 4e.

## Browser gate (`fleet-window-bombardment.json`)

The new fixture scenario `FleetBombardment` (code 67) sets up the player's
fleet over a system of the other side. That system's facilities are moved
elsewhere, and two of the other side's regiments stand on its planet. The
fleet's ship belongs to the player's first class that bombards, carries
regiments and has a hyperdrive.

The `bombardment` case of `tools/interface-parity/fleet-window.mjs` runs
these steps:
1. Right-click the fleet icon. The menu lists Planetary Bombardment, and no
   submenu is shown yet.
2. Click the parent. The menu stays open.
3. The submenu appears 122 ms (Alliance) and 139 ms (Empire) after that
   click. The timer runs from the moment the menu is shown, as the
   original's does.
4. The four rows appear on the parent's left, overlapping it by three
   pixels:
   - Alliance: menu x 347, submenu x 188.9 = 347 - 161.1 + 3; submenu y
     114 = 75 + 42 - 3;
   - Destroy System is disabled, because the fleet has no Death Star.
5. Click General Bombardment. The game reports "Orbital bombardment at
   Akrit'tar": 2 shots for the Alliance and 4 for the Empire, with 0
   targets destroyed. The regiments' bombardment defense beat each draw.
   Screenshots: `alliance-bombardment-submenu.png`,
   `empire-bombardment-submenu.png`, `*-bombardment-done.png`.

The whole Fleet window suite (46 cases, both sides) passes, including the
pixel comparisons of `chrome`. Other suites that open the object menu:
- `build-selection`, `status-window` and `sector-quadrants` pass.
- `fleet-move` passes except `confirmed-move` (both sides), which fails at
  its first line. There, `probe` finds no bare map point from the setup's
  system positions, before any menu step; this stage changes neither.
- `mission-dialog` was not run, because REBEXE.EXE cursor 1002 is not
  staged on this machine.

The suites ran under Arch Linux Chromium 153.0.8010.52 through a temporary
copy of each script that read a Linux browser manifest. The pinned Chrome
for Testing 151 (macOS) is not installed here. The copies were removed and
the manifest is unchanged. The test WASM SHA-256 was
`3d04f0226900af084fca2d1a1a47a2344c7f6e282b36dd71fb3f2636aee06612`.

## Browser run

The run used the release build with `agent-browser` (headless Chromium, 640
by 480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast. Each
frame's work time was measured by wrapping the game's
`requestAnimationFrame` callbacks:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3600 | 0.5 ms | 0.7 ms | 22.1 ms |
| Fast | 60 s | 3601 | 0.6 ms | 1.5 ms | 3.1 ms |

- No frame took over 50 ms.
- The AI's longest day took 17 ms (day 1).
- The Fast run's screenshot shows day 152 (`2-fast.png`).
- The console, read a little later, has 164 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Campaign (`playtest-bombardment-menu.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`: the event
and AI action counts match stage 4e's exactly.

## Gates

- `cargo test --workspace --no-fail-fast`: 2208 passed, 0 failed.
  - `game_menu.rs`:
    - the submenu's placement, including both strict edges;
    - the timer's ticks and the arrows;
    - three timed runs through egui: hover then the timer then a child
      chosen; a click on the parent or a disabled child; another highlight,
      and Escape.
  - `object_menu.rs`: the four targets by their gates, the rows of an open
    menu, and the parent enabled in the fleet and icon menus.
  - `main.rs`: the order's team.
  - The fixture: the scenario's world and its report.
  - Each new behaviour fails with its change neutralized (14 cases).
- Tests that changed: the fleet and fleet icon menus' enabled rows now
  include Planetary Bombardment (`FUN_00442590`: a parent is enabled);
  the fixtures' `OrderGates` literals gain `bombard`.
- `cargo mutants --in-diff`: the first run had 200 mutants, 29 missed and
  26 unviable. After the new tests, a second run of the 71 mutants in the
  affected functions caught all 71. The 15 left are not covered by a
  headless test:
  - drawing and loop functions: `main`, `apply_panel_action`,
    `draw_agent_menu`, `draw_bombardment`. Of these, the side test in
    `main`'s command and in the order's handler (`== Alliance`) runs in the
    browser gate for both sides;
  - the arrow's `6.0 * scale` offset: the tests have no bitmaps, so the
    arrow is not drawn. The offset is visible in the screenshots;
  - the parent's highlight while its submenu is open, and the repaint
    requests: these change drawing and scheduling only;
  - `ObjectMenuState::submenu_shown` as `false`: the browser gate checks
    the submenu's absence, then its presence;
  - `!children.is_empty()` in `draw_object_menu`: this is equivalent,
    because only a fleet's or a fleet icon's menu has the parent, and both
    have children.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.
- The replay equivalence check passes with 0 browser errors. Golden
  `v1:b160883b9ed0c807`, final `v1:f5933994514e98bc`, both unchanged.
  WASM SHA-256
  `705dba2428e2f8b1bd7688c530cc2d9e8f905348896e220d928557cb2ed830e3`.

Hashes are in `SHA256SUMS`.
