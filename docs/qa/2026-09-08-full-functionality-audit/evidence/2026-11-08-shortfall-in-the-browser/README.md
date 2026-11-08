# Maintenance Shortfall in the browser, against A0

Date: 2026-11-08 (night). Cells:

- EVT-01-C006 (shortfall)
- CMD-01-C006 (letterboxed widescreen)

## Maintenance Shortfall

`maintenance_message` (b113) already wrote this message as the original
does. It was checked by unit tests only. The A0 state is
`.artifacts/a0-vm/maintenance-shortfall/test-day1224-corvette-no-system.png`
(unedited Alliance game, day 1224), which shows a corvette lost in
hyperspace:

- "The following units have been destroyed at  due to a maintenance
  shortfall:" over "Corellian Corvette 2";
- picture 0x3f5;
- Display lit, its target unplaced.

The EventMessages fixture now files that message through the same
`maintenance_message`, with no system, before its other Manufacturing
reports (`interface_test_fixture::maintenance_shortfalls`). It lists under
"Construction Yard Idle on Womrik", which stays first as A0
manufacturing-messages/q01 shows.

`message-display.mjs` (Linux) opens it from the Manufacturing list. It
checks:

- the title, and the text: no system's name, the ship's own name;
- the picture 1013 (Alliance) / 1014 (Empire) at the window's (12, 33).

Against A0 (Alliance), 0 pixels differ in each of:

- the picture;
- the title's image (15 × 16);
- the rail (414..468 × 11..296), with Display lit.

## Letterboxed widescreen

This needs no new run. The Linux gates already run every GID scenario and
every tactical case at 1280x800 letterboxed as well as at 640x480, for
both sides:

- `run.mjs --all` in batch b128 (run 2026-11-08T01-42-36-919Z-2428737): 38
  GID 1280x800-letterboxed executions pass.
- The tactical matrix passes 148 of 148, half of them letterboxed.

Their probes place each click and pixel check at the centred uniform
scale (`offsetX = (width - 640 × scale) / 2`), so the letterboxed layout
is exercised throughout. The original runs at 640x480 only, and A0
confirms the 4:3 content. The gates do not assert the black bars' colour
on their own.

## Navigate (EVT-01-C021)

The ledger said "not compared in A0", but the comparison has existed
since b111. `message-display.mjs` presses Display on a placed message,
finds the index closed (FUN_00467f10), and compares the sector window it
opens with A0 message-rail/display-sesswenna: the window's left edge, its
bottom edge and its switch-side key are 0 pixels off (Alliance, Linux).

## Runs

Batch b129: 23 of 23 gates, Linux, Arch Chromium 155. The change touched
only the fixture and the gate, so no tactical matrix ran.
