# The tactical right-hand panel follows the original's captures (P24)

Date: 2026-10-24.

## Source

The first lossless A0 captures of the original `REBEXE.EXE` (SHA-256
`b3fe3997...ed6ab`), taken in a Windows 10 22H2 (build 19045) QEMU/KVM guest.
The guest display mode is the game's own 640x480; the screenshots are QMP
`screendump` PNGs with no host scaling. GOG's DDrawCompat wrapper gets
`DisplayResolution=app`; `REBEXE.EXE` itself is unchanged. The captures and
their manifest stay in the ignored `.artifacts/interface-parity/a0/` store
(94 of 106 cells so far). Only cell IDs are cited here.

Each TACTICAL.DLL bitmap was located in a capture by exact pixel match,
leaving out its transparent key colour:

- All the 149x236 right-hand panels (1302-1310) sit at (482, 28), with a
  match of up to 1.000 (TAC-04-C005, TAC-01-C002, TAC-05-C012, TAC-06-C003).
  The port drew them at (481, 27). Their buttons and icons are one pixel
  further right and down as well: capital navigation 1101/1103, order
  buttons 1105/1107, maneuver buttons 1133-1141 and 1117, assign 1113/1115,
  and subsystem icons 1201-1225.
- With nothing selected, the panel is the player's emblem: 1303 for the
  Alliance (TAC-04-C005), 1304 for the Empire (TAC-01-C002). The port drew
  the blank 1301 and used 1303/1304 as the Battle Options panel.
- The Battle Options panel is 1306 for both factions (TAC-06-C001), and
  TACTICAL.DLL has no second one. Its buttons sit at Withdraw (500, 47),
  Get Immediate Result (496, 121), Observe/Take Command (569, 121), Game
  Options (505, 212) and Close (579, 212). The port's positions were
  3-14 px off.
- The left order button (art 1107, tooltip "Maneuvers Tactics") opens the
  maneuvers panel, and the right one (art 1105, "Missions") opens missions
  (TAC-05-C012, TAC-05-C015). The port had the two swapped, and the
  resource names too.
- The selected ship's meters. In the run of TAC-03-C006, C005 and C003,
  the right meter emptied first and the ship died when the left one
  emptied, so the left meter is hull and the right one shield. That is the
  reverse of the port's P58F4 reading. Both are 37x10, at (516, 109) and
  (585, 109). The remaining part is pure blue (0,0,255) and the lost part
  pure red (255,0,0) for both factions (TAC-03-C009, Imperial). The red
  that P58F4 took for the Empire's hull colour was the lost part.
- The capital portrait (2007, 2026) matches every opaque pixel at
  (508, 45). The port drew it at (507, 37).

## Change

- `TACTICAL_PANEL_X/Y` = (482, 28) for every right-hand panel. All panel
  controls, icons, portraits and texts move by (+1, +1). The portrait goes
  to (508, 45).
- `tactical_display_panel_resource` picks the emblem with no selection.
  `BATTLE_OPTIONS_PANEL` = 1306.
- The order buttons and their resource constants are swapped:
  `BTN_MANEUVERS_TACTICS_*` = 1107/1108, `BTN_MISSIONS_*` = 1105/1106.
- The meters: hull on the left, shield on the right, blue remaining and red
  lost. Rounding to whole pixels is `port:`.
- `run.mjs` expectations move with the panel, and each change cites its A0
  cell. The hull check now looks for pure blue for both factions. The
  portrait matte probe moves to the portrait's bottom-left pixel, because
  the ship name covers its top row.

## Gates

- Workspace: 2734 passed, 0 failed.
- New tests:
  - `selected_panel_controls_sit_at_the_a0_panel_origin`
  - `no_selection_shows_the_players_emblem_panel`
  - `selected_meters_show_hull_left_and_shield_right`
  - `meter_remaining_part_is_rounded_and_clamped`
- Changed expectation: `tactical_assignment_hit_regions_follow_the_source_panels`
  now has the left button opening Maneuvers (A0 TAC-05-C012/C015).
- Mutants, with both packages' tests: 27 tested, 7 caught, 2 unviable,
  18 missed. All 18 missed mutants are in the macroquad draw functions
  (`draw_tactical_selected_meter`, `draw_tactical_assignment_panel`,
  `draw_tactical_withdraw_confirmation`, `draw_original_tactical_hud`),
  which the unit tests cannot run without a GPU. The browser suite's pixel
  probes cover them: exact panel, button, meter and portrait pixels at
  640x480.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `4f0aba3f0ef7a57042296eed094539cfccf06873b5a2216d4dca5f53bc7d769a`.

## Still open from the same captures

- The lower HUD: the navigation set, zoom, camera sphere, recall/memorize
  (bitmaps swapped), pause, highlight and options buttons are 1-25 px off.
- The group bar: task-force slots are 29 px apart, not 26. The fighter
  header sits at 296 and the squadron buttons at 347 + 29k.
- The texts in the panel: the original uses a proportional font, at other
  positions.
