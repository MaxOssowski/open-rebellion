# When the sector window redraws 904 after an HQ move (2026-11-12)

Trace only. No code, ledger or gate changed.

## What was asked

Ledger cell OBJ-13-C006 ("HQ in transit") is partial. Its note says an open
sector window in A0 kept 904 over the origin after a move, while the port
redraws at once, and that the refresh was untraced
(`../2026-11-04-headquarters-move/`, `../2026-11-11-hq-and-tactical-states/`).
The question: what makes the original redraw the HQ mark after a move, and
does the port differ.

## Trace in the original

The move does send a notice, and the notice refreshes both systems.

1. **The move.** `FUN_00556390` calls the HQ's slot `+0xa8`. In the HQ
   vtable `0x0065f280`, `+0xa8` (`0x0065f328`) is `FUN_004f8630`, which
   calls `FUN_004f6fd0`, the generic set-parent. It moves the master object,
   then mirrors the move into the side copies 1 and 2 (`FUN_00539fd0(1|2)`,
   `FUN_004f9d40`).
2. **The parent notice.** `FUN_004f9d40` reparents the side copy
   (`FUN_004fc600`), then calls slot `+0x15c` with (old parent, new parent,
   ctx), at `0x004f9e9f`..`0x004f9ea4`. It does so only when `FUN_004fd3d0`
   and `FUN_004fd600(side)` allow it. In the HQ vtable, `+0x15c`
   (`0x0065f3dc`) is `FUN_004fbc20`. It logs `GameObjParentNotif` /
   `Parent` and posts UI notice `0x300` with `FUN_0053fe40` (`0x004fbce2`).
   `FUN_0053fe40` stores the old parent's key at `+0x24` and the new one's
   at `+0x28` (`0x0053fe8b`, `0x0053fe98`). It queues the notice through
   `FUN_004fd3b0` (hook `DAT_006b2ad4`).
3. **The UI message.** `FUN_0041e580` registers `0x300` to `FUN_0043e540`,
   class `0x00659698`. Its `vt18` (`FUN_0043e5b0`) calls the player's slot
   `+0x14c` with (object, old, new). For the local player
   (`Class_0065ad70`) that slot is `FUN_00486d80`. It calls:
   - `FUN_0041d4d0(object, 0, old)` → `FUN_0042aaf0`, which calls slot
     `+0x60` of each window that watches the object or the old parent;
   - `FUN_0041d4b0(object)` → `FUN_0042a900`, which calls slot `+0x5c` of
     each window that watches the object's new container, when that
     container is a system (`0x90..0x97`).
4. **The sector window.** Class `0x00659f18`:
   - `+0x60` is `FUN_0045b690`. When the old parent is a system, it calls
     slot `+0x64` for that system. This is the origin.
   - `+0x5c` is `FUN_0045b3e0`. For the HQ's family `0x20..0x21`
     (`0x0045b5c5`..`0x0045b60b`), it calls slot `+0x64` for the HQ's
     container (`+0x1c`). This is the destination.
   - `+0x64` is `FUN_0045b770`. For a system key it runs `FUN_0045bbb0`.
     Slot `+0x64` itself ignores an HQ key: its ranges skip `0x20..0x21`.
5. **The overlay.** `FUN_0045bbb0` keeps one HQ overlay, id `0x10000`. On
   the origin it drops the overlay when the HQ no longer stands there. On
   the destination it reuses the overlay, or creates it, and sets its rect
   over the destination planet. So both refreshes come from the one parent
   notice.

What is not traced:
- When the queued notice reaches the window. The notification queue
  `+0xa4` is drained in the frame step (`FUN_0041dff0` phase 5,
  `FUN_0051df30` → `FUN_0051dd20`, and phase 6 `FUN_0051e050` →
  `FUN_0051eaf0`). Whether that step runs while the game is paused is not
  traced.
- What the per-side flag `FUN_004fd600(side)` reads (`ctx +0xc` / `+0x10`).
  If it is clear, the side copy moves with no notice.

## The A0 captures again

`.artifacts/a0-vm/headquarters-move/` (day 555 and 556, Moddell sector):

| Capture | SHA-256 |
| --- | --- |
| `00-now.png` | `b370ea313cad1d762674bb635faadd45702d3cfcd51c45f5b53aecb648fc791f` |
| `07-closed-status.png` | `6ea3be05e18669e4ef183a4db5c3f4a45ab1405786ef97930731c73e1353766a` |
| `09-reopened.png` | `444f689f85fc2c148a5b9b464abd215a7ea5b9a451660d9ee8ed4b180cfcda73` |

- `00` (day 555) and `07` (day 556) are the same around every planet I
  compared: Agrilat, Basilisk, Annaj, Adega, Pzob, Gandolo IV (0 pixels
  differ).
- `09` is the same day, 556, after the window was closed and opened again.
  904 moved from Pzob to Gandolo IV. Other systems also changed: Agrilat
  (80 pixels), Basilisk (80) and Annaj (286) now show a red side icon that
  `07` lacked.
- So in `07` the whole window was stale, not only 904. The window had been
  open since before day 555, and the HQ's move happened at an unknown time
  (the 2026-11-04 README). These captures do not show a window that missed
  the HQ's move notice. They show a window that missed several notices.
- The earlier note "hyp: the move sends no refresh" (`ghidra/notes/
  headquarters-move.md`) is refuted by step 2 to 4 above.

## The port

`crates/rebellion-render/src/sector_window.rs` reads
`shows_alliance_headquarters` from the world on every frame (line 962). The
HQ flag moves at once in `rebellion_core::headquarters_move`. So 904 moves
in the frame after the order. This matches the traced path, where the same
move posts the `0x300` notice that refreshes the origin and the destination.

## Verdict

- No port bug is shown. The original refreshes the HQ mark from the
  `0x300` parent notice of the move itself (`FUN_004fbc20` →
  `FUN_00486d80` → `FUN_0045b690` / `FUN_0045b3e0` → `FUN_0045b770` →
  `FUN_0045bbb0`). The port's immediate redraw matches it.
- The A0 stale mark is not evidence against this. In `07` the window was
  stale for other systems too, and the move was not seen.
- Two things stay open: whether the notice waits while the game is paused,
  and what `FUN_004fd600(side)` gates. The code does not settle either.
  Until an A0 capture does, the cell stays partial. No change to the port
  is proposed.

## A0 capture needed

In the VM's Alliance game:

1. Open the sector window that holds the HQ, with a second Alliance system
   of that sector visible.
2. Pause. Move the HQ to the second system (menu Move, then a click on the
   planet). Capture at once, with the window open and the game paused.
3. Wait 2 s, still paused. Capture again.
4. Unpause until the day changes. Capture.
5. Repeat 2 to 4 with the game running at normal speed.

If 904 moves in step 2 or 5, the port is right. If it moves only in step 4,
the notice waits for the clock, and the port should hold the old mark while
paused.

## Batch b198

The full gate batch b198 (2026-11-12 10:16-11:30) passed 29 of 30 gates, the workspace (3620 tests) and fixture tests (158), and 172 tactical battle checks, 0 failed. The one failure was fleet-window.mjs fleet-encyclopedia, which still expected the old fleet object 0x0800_0000; the expectation now names the class 0x0800_0004 (FUN_004f7d50) and both sides pass (run fleet-window-2026-11-12T10-36-04-649Z-764990).
