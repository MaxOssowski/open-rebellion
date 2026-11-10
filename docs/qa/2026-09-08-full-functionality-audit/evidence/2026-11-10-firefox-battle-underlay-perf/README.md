# Firefox frame rate: the battle underlay is copied once

Date: 2026-11-10. A regression from `3c842e33` (2026-11-09, "the Battle
Alert and Summary sit on the live command center").

## The report

The owner played the browser build in Firefox, with the GPU in use, and
found it very slow.

## Measurements

Headless Firefox 158 on the owner's machine (RTX 3060), the Empire's
Fleet window fixture at Coruscant (`?fixture-code=612`), 1366 by 682.
A probe injected into the served page timed `requestAnimationFrame` for
six seconds and wrapped every WebGL call
(`target/claude-tmp/keep/ff-probe.mjs`, not in Git).

| Build | fps | Median frame |
| --- | --- | --- |
| before | 3.2 | 300 ms |
| before, `copyTexImage2D` skipped in `gl.js` | 60 | 17 ms |
| before, `copyTexSubImage2D` into the same texture | 3.2 | 317 ms |
| before, `preserveDrawingBuffer: true` | 2.8 | 350 ms |
| after | 60 | 17 ms |
| main menu, before | 60 | 17 ms |

Before the change, `gl.finish()` (macroquad's end of frame) took about
340 ms a frame, and the game's own callbacks about 4 ms. Chromium ran
the same scene at about 50 fps even with software WebGL, so the
Chromium gates never showed it.

## The cause

`main.rs` copied the whole screen into `strategy_underlay`
(`Texture2D::grab_screen`, `copyTexImage2D` from the default
framebuffer) at the end of every galaxy frame. Firefox stalls on that
read of the drawing buffer.

## The change (`main.rs`)

- The copy is made once, at the end of the frame a battle begins in.
  When the day's tick enters a battle from the galaxy, the frame still
  draws the galaxy (`battle_underlay_pending`). At its end the screen is
  copied and the battle takes the screen. `port:` this one frame stands
  in for the original's live command center under the alert.
- In that frame the ending check and the story movies wait, so nothing
  else takes the screen first.
- If that frame left the galaxy for the cockpit's Game Options, the
  battle still takes the screen, and the windows use the plain backdrop
  instead of an older battle's copy.
- The frame keeps the last frame's message rail. A0
  `empire-game3/battle-alert-coruscant` shows no message of that day lit
  beside the alert. Without this, the battle's day lit the Empire emblem
  control and `fleet-window.mjs squadron-battle` found the rail 193
  pixels off A0.

## Tests

- `fleet-window.mjs squadron-battle` (Empire): the rail beside the
  Battle Alert matches A0 in every pixel again.
- `rebellion-app` has no unit test of the frame loop. The Firefox probe
  above is the measurement.

## Batch b179

23 of 23 interface gates on Linux, `fleet-window`
(`fleet-window-2026-11-10T20-34-36-714Z-1839107`) among them;
workspace tests 3507 passed, fixture tests 140. Tactical matrix
`2026-11-10T21-31-26-850Z-1920246`: 148 of 148. A first batch, b178,
was stopped after `fleet-window` found the rail 193 pixels off A0; the
rail fix above followed.
