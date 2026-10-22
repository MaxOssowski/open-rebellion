# The Battle Summary's Display page (F-006 / TAC-07)

Date: 2026-10-22. The reading is in `ghidra/notes/battle-summary-window.md`,
"The Display page".

## Change

- `tactical_view.rs`: `battle_display_lines` fills the message line
  (`+0xac`) and the instruction (`+0xb8`) as the report builders do:
  `0x7012` and `0x7013` for the viewer's win (`FUN_0049b330`), `0x7011`
  and `0x7013` for its withdrawal, otherwise `0x7010` and `0x7014`
  (`FUN_0049b150`, `FUN_0049b4d0`).
- `battle_display_page` draws them at (24, 40) and (24, 130), 328 wide,
  in the viewer's colour (`FUN_0046f360`). The system button sits at
  (29, 182). The fleet button sits at (229, 182), with art by the
  viewer's side (10765 / 10766), only while the viewer's fleet survives.
  Both press to 10767 / 10768. The labels sit under the buttons at y 278:
  the system's name and "Fleet" (string `0x1942`).
- Before this change the port drew its own "Go Directly To..." heading
  and the winner's fleet button, labelled "Alliance Fleet" above the
  buttons. The fleet button now opens the viewer's fleet.
- `run.mjs`: the background probe's ignored bands move to the original
  text positions (message, instruction, labels).

port: the withdrawn fleet's destination (`+0x48`) is decided after the
report, so `0x7011` drops " to {3:1}". The port draws font 7 (18 px bold)
at the port's font-6 size, without bold.

## Gates

- Tests: 3 new. Workspace 2628 passed, 0 failed.
- Mutants: 25 caught, 8 unviable, 4 missed. The missed ones are the
  mouse-to-window subtraction in the macroquad wrapper. The browser probe
  clicks the system (Alliance) and fleet (Empire) buttons and checks the
  route log.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build):
  tactical `production-battle-results-presentation` passes 4 of 4 cases.
  `alliance-display.png` and `empire-display.png` are the 640x480 pages.
- Replay equivalence passed.
- Release WASM `b83221d29aa6e1108afa7be4329ab905500a89f516c9ce56868c55829e03f0e6`.
