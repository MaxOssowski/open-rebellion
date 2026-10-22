# The Status window picture's marks (UI, P08/P09)

Date: 2026-10-22. The reading is in `ghidra/notes/status-window.md`
("Port notes": `FUN_0042c3b0` read in full).

## Change

- `status_rows.rs`: `StatusView::marks`, what `FUN_0042c3b0(.., 1, 1)`
  draws under the picture:
  - a regiment's backdrop, STRATEGY 11514;
  - en route (`+0x50` bit `0x10`): GOKRES portrait `+0x1000` for a ship
    or a squadron; STRATEGY 11516 for a regiment (GOKRES for
    `0x10000002`/`0x10000008`); 11520 for a special force (GOKRES for
    `0x3c000003`/`0x3c000005`); 10426 / 10476 for a fleet;
  - damaged (bit `0x200`): GOKRES portrait `+0x2000` for a ship.
- `status_window.rs`: the marks are painted under the picture, at its
  top-left and clipped to its size (`picture_layers`).
- `bmp_cache.rs`: the marks are keyed, as the picture is (STRATEGY 11514,
  11516, 11520; GOKRES `0x2640..0x278f`).

port: no unbuilt mark (every port object is built); no fleet `+0x200`
mark (its writer is untraced); facilities are never en route.

Also in this run (commit `ba8e8a90`): the Assault Summary window blocks
the galaxy view's pointer over its rectangle.

## Gates

- Tests: 6 new or extended. Workspace 2609 passed, 0 failed, before the
  keying fix; render tests pass after it.
- Mutants (status rows and window diff): 16 caught, 0 missed, 11
  unviable. The pointer-block helper: 4 caught.
- `provenance-scan` 0 uncited.
- Browser, Arch Chromium 155.0.8059.84: Status window 10 of 10 (a first run
  failed with an unkeyed blue matte on the new marks, fixed by the keying
  above); Fleet window 58 of 58. Interface WASM
  `13cfb497b3486ae6a2d73253501bb7e338d60790b27b27e29017393885b6cb13`.
  `*-capital-ship-window.png` shows the fixture's damaged ship with its
  damage mark; `*-regiment-window.png` shows the regiment on 11514.
- Replay equivalence passed.
- Release WASM `09f5281093278ad4372b4bfc94f0ca4e72660fb0a2ba6041f670804f17f386aa`.
