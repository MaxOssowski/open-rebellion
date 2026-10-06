---
title: "Galaxy View Input: a Fixed Map"
description: "Which mouse messages the galaxy view's procedure FUN_00422ce0 handles, and why the star map neither zooms nor pans"
category: "ghidra"
created: 2026-10-06
updated: 2026-10-06
tags: [galaxy-view, input, camera]
---

# Galaxy view input: a fixed map

Read 2026-10-06 from `FUN_00422ce0.c` (the galaxy view's window procedure,
Ghidra 12.1.3 headless, read-only project).

## The messages it handles

- `WM_MOUSEMOVE` (`0x200`): records the point and returns.
- `WM_LBUTTONDOWN`, `WM_LBUTTONUP` (`0x201`, `0x202`), including the drop
  of a drag in mode 2 (`move-order.md`).
- `WM_LBUTTONDBLCLK` (`0x203`), resolved through `ChildWindowFromPointEx`.
- `WM_RBUTTONUP` (`0x205`): the object menu (`object-popup-menu.md`).
- The view's own `0x214` and `0x240`, and `WM_COMMAND`
  (`fleet-finder.md`).

There is no `WM_RBUTTONDOWN` (`0x204`) handler and no `WM_MOUSEWHEEL`
(`0x20a`) case, and the mouse-move path changes no offset or scale. **The
original star map neither zooms nor pans**: every system keeps one screen
position for the whole game.

## Ported

- `rebellion_render::galaxy_camera` gives one framing at every display
  scale: centre `GALAXY_CAMERA_CENTER` (450, 470) at logical zoom 1.
  `GalaxyMapState` keeps no camera.
- hyp: the centre and scale are the port's own framing; the original's
  system-to-screen offsets are not traced.
- The fixture keeps the Pan (27) and Zoom (28) scenarios, which the catalog
  validator requires. `tools/interface-parity/run.mjs` asserts that the
  wheel and right drag leave the frame unchanged.
