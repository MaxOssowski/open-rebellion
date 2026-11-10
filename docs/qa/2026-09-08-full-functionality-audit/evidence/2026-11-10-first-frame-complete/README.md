# The first frame is complete

Date: 2026-11-10. Covers CMD-01-C004 (first frame complete).

## A0

A new game's first frame in the original: `.artifacts/a0-vm/empire-initial/00-new-empire-game-day2-message-index.png`
(SHA-256 in `../2026-11-10-empire-initial-frame-a0/SHA256SUMS`). The
Message Index stands over a complete cockpit. Every rail, aperture and
console is already drawn. The settled frame after the index closes
(`01`) has the same shell, pixel for pixel
(`2026-11-10-empire-initial-frame-a0`).

## The browser check

`run.mjs` (the GID gate) now captures the first frame the game shows
once it reports ready, before fonts settle or anything else runs. The
new `verifyFirstFrameComplete` compares that frame with the settled
frame in three shell regions at 640 by 480:

- the left column above the advisors (x 0..54, y 40..299);
- the right column (x 540..639, y 40..389);
- the bottom console right of the advisors (x 120..639, y 395..479).

All three must be identical. The galaxy aperture, the top readouts and
the animated advisors are left out. The first frame is saved as
`first-ready.png`, and the result appears in each case's `first_frame`.

Run `2026-11-10T06-47-01-846Z-4111588` (Linux Chromium, all GID
scenarios), 152 of 152:

- in all 76 native cases (both sides, every scenario), all three regions
  have 0 differing pixels;
- the 76 letterboxed cases are skipped as non-native scale.

The check cannot fail on the current build. The port draws the cockpit
only once loading ends, which is the CMD-10-C010 note, and it reports
ready after that. The check guards that order.

## Batch b169

23 of 23 interface gates on Linux, the GID gate `run`
(`2026-11-10T07-19-29-407Z-4192010`) with this check among them;
workspace tests 3498 passed, fixture tests 139. Tactical matrix
`2026-11-10T07-49-00-880Z-42959`: 148 of 148.
