# Native check of the player's orders (partial)

Date: 2026-11-28. These are agent checks, not human ones. Roadmap M1 and
the tracker item "Natywne sprawdzenie rozkazów gracza" ask that Move,
Confirmed Move, the troop load through the Fleet window and the mission
entry, which pass the browser gates for both sides, be checked in the
native Linux window.

## Setup

- Machine: CachyOS Linux, Hyprland (XWayland), NVIDIA driver.
- Driver: `cua-driver` 0.31.0, daemon in its standard permission mode,
  background X11 delivery only (the user's pointer never moved).
- Game: `target/release/open-rebellion`, SHA-256
  `66d6c8113cab3b3d4d576357c3b476e5184ddea717a8e45f1d663824e58e70bd`,
  built from `3f72dcec` (the dirty tree is the untracked web WASM only).
  Launched muted with `scripts/launch-native.sh`, seed 42, Alliance,
  "Easy, Small Galaxy, Standard Game". The window was 948 by 1026.
- Commands (`--commands`) reached a state only: `Start game: Alliance` and,
  in a discovery run, `Open fleet icon menu: <system>` for every system,
  which showed fleet icons at Corfai, Hoth, Praesitlyn and Yavin.
- The screenshots show original artwork, so they are not committed; their
  hashes are in `screenshots.sha256`.

## Move: passes

Driven with `cua-driver`:

1. A right click on Fleet 1 in the Yavin Fleet window opened its menu
   (Move, Confirmed Move, Planetary Bombardment, Planetary Assault, Rename,
   Encyclopedia, Status, Scrap) (`discover2/yavin-menu.png`).
2. Move turned the pointer into the targeting cross (`discover2/move-4.png`).
3. A press on Gamorr in the Dufilvan sector window, a short pointer move
   and the release gave the order. Fleet 1 left the Yavin window on day 22
   (`discover2/t-3.png`), and the Fleet Finder's Display opened the Gamorr
   window for it (`discover2/t-9.png`).
4. On day 103 the fleet icon stood at Gamorr and the system showed its
   values (`discover2/move-arrived-day103.png`).

While the fleet travelled, Gamorr was unexplored for the Alliance, so the
Gamorr window listed nothing (`visible_fleets` gates on `side_explored`);
on arrival the copy went live and explored. Whether the original lists an
own fleet in hyperspace at an unexplored destination was not checked.

## Not checked: Confirmed Move, the troop load, the mission entry

The background input cannot place a target reliably. The game reads the
targeting release from egui's latest pointer position (`main.rs`
`targeting_held && is_mouse_button_released`, `pointer.latest_pos()`), and
`cua-driver`'s background clicks and presses do not move that pointer
reliably:

- a plain `click` on a menu item first only hovers it, a second click
  chooses it;
- the target pick needs `mouse_button_down`, `mouse_drag` and
  `mouse_button_up`, and even then the release was read at an earlier
  position. A temporary trace (not committed) logged a Move released on
  Selonia as
  `destination=SystemKey(100v1) join=Some(FleetKey(2v1)) checked=Err("a fleet cannot move into itself")`:
  the point of the earlier right click on Fleet 1's own entry
  (`alliance-confirmed/a-13.png`).
- Three Confirmed Move attempts chose their target frame (Toprawa, Ord
  Mantell, Kashyyyk, Selonia) but issued no order.

A real mouse sends motion before every press, so this is a limit of the
harness, not a finding against the game. `cua-driver`'s `drag` tool
(a uinput pointer) was not used on the user's desktop. A nested X server
with `xdotool` (neither installed) would give real pointer motion without
touching the desktop.

## Finding: abort at exit on NVIDIA

Quitting through the main menu (Escape from the galaxy, then Escape) ends
with `*** buffer overflow detected ***: terminated` after
`[quit] audio_stopped=true cleanup=complete`, every time (2 of 2). The core
dump's stack:

```
__chk_fail <- libxcb.so.1 xcb_get_extension_data <- xcb_send_request
<- xcb_dri3_free_syncobj <- libnvidia-egl-xlib.so.1 <- libEGL_nvidia.so.0
<- __cxa_finalize
```

NVIDIA's EGL library frees its DRI3 sync objects in its exit destructor
and sends an X request on a connection that is already closed. The game
has finished its own cleanup by then (saves are written), but the process
dies with SIGABRT and a core dump. The process held 97 descriptors, so it
is not the `FD_SET` limit. A fix (end EGL before the X display closes, or
leave with `_exit` after cleanup) is a platform choice left to the owner.

## Conclusion

Move passes natively. Confirmed Move, the troop load and the mission entry
stay unchecked in the native window: the harness cannot place a target
reliably. The item stays open.
