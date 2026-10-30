# An open Finder is modal (CMD-04-C017)

Date: 2026-10-30. Surface ledger CMD-04, "Finder windows are modal",
which the reconciliation left failing: the port's Finders blocked the map
only under their own rectangle, and the cockpit stayed live.

## A0 (Windows 10 VM, Empire, day 210)

Captures are in `.artifacts/a0-vm/modal/`; hashes are in `a0-captures.sha256`.

With the Fleet Finder open (F3, `01-f3.png`), none of these does anything;
the screen stays as it was:
- clicks on the GID control (`02-gid-click.png`);
- a right click on the agent droid (`03-agent-right.png`);
- clicks on the Encyclopedia control (`04-ency-click.png`);
- clicks on a rail button (`05-rail-click.png`).

The difference images show only the pointer and the droid's animation.
F4 replaces the Fleet Finder with the Troop Finder (`06-f4.png`). There
the close button shows a "Close" tooltip, which the port does not draw;
that is noted as a separate gap.

Not traced: the mechanism. `FUN_0042a0c0` creates the Finder (window type
`0x15`) and clears bit `0x10000000` of its `+0x50`; how the frame then
refuses the other windows' input is not read.

## The port (`main.rs`)

While any Finder is open (`finder_modal`):
- the map takes no pointer;
- the strategic input that gates the GID control and menu, the speed menu,
  the droids' menus, the rail's Message Index command and the object menus
  is off;
- the cockpit's primary controls take no click.

The Finder's own controls keep their input.

## Browser (Arch Chromium 155, interface WASM `fe4f1c50…`)

`fleet-finder.mjs` passes every case for both sides
(`fleet-finder-result.json`). The new case `modal` opens the Finder,
clicks the GID control and right-clicks the agent, and checks that neither
the GID menu nor the Agent menu opens and that the Finder stays open. Run on
the site built before this change, it failed: "the GID control opened its
menu under the Finder".

The gate had not been run since 2026-10-29. Its expectations are brought up
to that day's A0-checked changes:
- **The name label's mask** is two rows taller. Font 4's "Ship Name" hangs
  its p to y 63; the font itself is from s203. The pixel count is 79,366.
- **A tab change** chooses the row the copied name best begins
  (`FUN_0046df90` case 100, `FUN_00609650`;
  `../2026-10-29-troop-finder-first-row/`). It no longer clears the
  choice.
- **The overlap case** drags the Fleet window over the Ship Finder button
  before reopening the Finder. Since s183 the window opens beside its
  icon, clear of the Finder's controls.

## Tests

`cargo test --workspace`: all pass. The change is the app's input
gating; the browser case covers it.
