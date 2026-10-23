# The Encyclopedia command opens the original window (P35)

Date: 2026-10-23.

## Before

The cockpit's Encyclopedia button and F7 (command `0x131`) only logged
`status=pending_original_window`; the routed original window (stage
`2026-10-23-encyclopedia-routing`) was reachable only from the object
menu, mission dialog, build selection and status window.

## Change

- `0x131` opens the original window when its source loaded (else the
  pending log stays); the rail's close `0xfb` logs
  `command=0xfb destination=encyclopedia status=closed`.
- The GID harness's Popular Support journey now expects
  `status=opened_original` for the button and F7, checks that the window
  appears, closes it with its `0xfb` control (hyp placement: centred,
  origin `(85,75)`), and requires the command centre to return
  byte-identical. The legacy `E` shortcut still fails closed.

## Visual check (browser, not committed)

On the test site, fixture 257 (Alliance galaxy), a click on the
Encyclopedia button showed the original index (title, Topic row, the
seven category icons, All Databases list from "A-wing"), matching
reference capture `steam-guide/guide-018`; the rail's topic control showed
the A-wing topic: title between the arrows (previous unavailable), the
ENCYTEXT body at the bottom. The EDATA picture was black: this machine
has no `EData` directory, so the runtime pack carries 0 Encyclopedia
images. Screenshot hashes (original artwork, kept out of Git):

- index `4dc2994418af242d8eaeb4ce5efd9a0389b390cf058e0b4cad822a4184d8b00f`
- topic `575cd5405602d2f8d40460e64b23343e921f1ecf50e8d95cb87830dbff902033`

## Gates

- Workspace 2704 passed, 0 failed.
- Mutants: the change is in the app's command routing and the harness;
  no library diff.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- Replay equivalence passed.
- Release WASM `82f0dddf8d6de70804e21875d30552911f2fe619404f328045b38d2ba715e5ef`.
