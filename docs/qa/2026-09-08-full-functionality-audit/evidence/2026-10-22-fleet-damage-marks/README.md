# A damaged fleet's marks (UI, P09)

Date: 2026-10-22. The reading is in `ghidra/notes/fleet-window.md`
("En route marks").

## Finding

`FUN_004feab0` sets a fleet's `+0x50` bit `0x200` while any of its
capital ships has it (`FUN_00501c00`: hull damage or a hit). The Fleet
window's entry (`FUN_004a37c0`) draws 10423 en route, 10424 damaged and
then its frame at (5, 5). Its picture panel (`FUN_004a5c00`) draws 10426,
10427 and then the picture 10425. The Empire's art is 50 higher. The port
drew the en route mark over the frame and had no damage mark. It also
left the Status window's fleet picture without 10427.

## Change

- `fleet_window.rs`: `fleet_damaged`, `fleet_entry_layers` and
  `fleet_picture_layers` (marks under the art).
- `status_rows.rs`: the fleet picture takes 10427 / 10477 while damaged.
- `tools/interface-parity/fleet-window.mjs`: the chrome case's expected
  window adds 10424 and 10427. The fixture's ship stands at 100 of its
  class's 2400 hull, so its fleet is damaged.
- One status test expectation of this night gains 10427, on the same
  evidence.

## Gates

- Tests: 1 new, 1 changed. Workspace 2621 passed, 0 failed.
- Mutants on the diff: 14 caught, 0 missed, 1 unviable.
- Browser, Arch Chromium 155.0.8059.84: Fleet window 58 of 58 with the
  new expectation (the first run, without it, differed in 18 and 31
  pixels: the marks under the frame). Status window 10 of 10.
  `*-actual.png` are the chrome case's windows.
- Replay equivalence passed.
- Release WASM `50179270a656c5db7b4ef22f0fe3018bb7153fcd40cf2c3a583d11cb20c8b8da`.
