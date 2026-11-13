# Quadrant icons: the facility icon's drop and its menu's Destination against A0

Date: 2026-11-13. Cells CMD-03-C019 (quadrant icon drag) and CMD-03-C018
(quadrant icon right-click menu). It follows `2026-11-13-icon-drops-a0`.

## A0

A0 is the original REBEXE in the offline Windows 10 QEMU VM. The game is the
unsaved Alliance game, days 377-390, at 640x480.

The captures are under `.artifacts/a0-vm/2026-11-13-facility-icon-drop/`.
That folder is ignored and never committed. Its `README.md` lists every
PNG, and its `SHA256SUMS` lists the hashes. The gate hashes the one file it
reads (`A0_FILES` in `tools/interface-parity/sector-quadrants.mjs`):

| File | SHA-256 |
|---|---|
| `08-facilities-icon-menu.png` | `56a2e41e827f3b41228565ac6af51477554cdfd95666735657c47e260fa641f5` |

These captures were read by eye, with the hashes from `SHA256SUMS`:

| File | SHA-256 | What it shows |
|---|---|---|
| `01-jerijador-facilities-window.png` | `d9eb730f…c9a0d` | all three areas show "Destination: Jerijador", the areas idle |
| `03`..`06` | `d61e8bc1…`, `b8e10508…`, `3e18ff98…`, `efdb79cc…` | the icon dropped on the HQ dome over Yavin's planet centre (410, 238) |
| `07-jerijador-facilities-window-after.png` | `821115dc…65c1` | still "Destination: Jerijador": the drop set nothing |
| `09`..`11` | `048bca19…`, `6ffbfc57…`, `979fcf0e…` | the menu's Destination clicked at (410, 238): all three areas show "Destination: Yavin" |
| `12`..`14` | `7acd5612…`, `9abb3121…`, `6eab576c…` | dropped on Jerijador's own planet: back to Jerijador |
| `15`, `16` | `6b969d97…`, `8f3df5d7…` | dropped on Yavin's planet edge (396, 252), off the dome: Yavin |

The menu in A0 08 sits at (225, 218) and is 90 by 102. Its rows are
Destination, Encyclopedia, Status (grey), Reserved and Scrap.

The two Yavin points sit on the HQ mark 904, whose top left is on its
planet's top left (FUN_0045bbb0):

- The dome point (410, 238) is 18 by 18 into the planet. Mark 904 is opaque
  there.
- The edge point (396, 252) is 4 by 32 into the planet. Mark 904 is keyed
  there.

## The original

The trace is in `ghidra/notes/sector-icon-menus.md` (the facility icon's
drop).

- **The drop.** `FUN_00422ce0` (window type 1) splits the kind 4 entry into
  a `0x214` order against the drop window's `+0x68`, the object under the
  point. `FUN_00537180` accepts only a system as a production manager's
  Destination; anything else is refused with `1`/`0x25`. `FUN_00487cc0` has
  no case for `0x214`, so nothing asks.
- **The HQ mark.** `FUN_0045cc10` walks the overlay list first. The
  headquarters overlay (`0x10000`) takes a point wherever its bitmap is
  opaque. So over the dome, `+0x68` names the HQ object (family `0x20`),
  and the drop sets nothing.
- **The menu's Destination targeting.** It reads `+0x70`, the planet, so it
  sets the system (A0 11).
- Not traced further: `FUN_0045c6b0`'s answer itself was not decompiled
  this session. That the HQ object is what it names is inferred from A0 07
  against A0 11 and 16.

## The port before

- The drop on the HQ dome set every area to the HQ's system. Gate run
  `sector-quadrants-2026-11-13T22-31-01-688Z-725956` (Alliance) logged
  `status=set areas=3 to=Fwillsving`.
- The port reads `+0x68` through the Mission release hit test, and that
  test treats the HQ as a flag on the system, not as an object.
- Everything else already matched A0, in the same run:
  - The drop on its own planet and on another of the player's planets set
    all three areas, with no question.
  - The menu's rows and frame matched A0 08.

## The change

- `crates/rebellion-render/src/sector_window.rs`:
  - Each pass records the headquarters overlays it painted.
  - The new `SectorWindowState::headquarters_at` answers the system whose
    mark is opaque under the point. It uses the same pixel test as a press
    (`headquarters_hit`).
  - Unit test:
    `a_point_on_the_headquarters_mark_names_the_headquarters_not_its_planet`
    (the mark's opaque middle names the HQ; the keyed planet corner names
    nothing, while `release_target` still gives the planet).
- `crates/rebellion-render/src/targeting.rs`: `destination_drop_target`
  takes the `BmpCache`. It returns nothing when the sector window under the
  point has the HQ mark there. `main.rs` passes `&mut bmp_cache`.
- `crates/rebellion-app/src/interface_test_fixture.rs`:
  - New fixture `QuadrantsFacility`: scenario 118, fixture code 119,
    appended last. It is `Quadrants` with a shipyard of the player's at the
    primary system, so its facility icon has managers.
  - The quadrant observation now reports `system_destinations`: each open
    System window's three "Destination: ..." lines.
  - Test:
    `the_facility_scenario_gives_the_primarys_facility_icon_managers_of_the_player`.
- `tools/interface-parity/sector-quadrants.mjs` has two new cases.
- **`facility-icon-drop`**, both sides:
  - The menu rows are Destination, Encyclopedia, Status (disabled),
    Reserved, Scrap.
  - On the Alliance, the menu is compared with A0 08, relative to its
    corner:
    - its size is 90 by 102;
    - the row kinds are white, white, grey, white, white;
    - each row's ink is within 1 pixel, and the right edge within 2 (the
      accepted text-width deviation);
    - the frame ring has 0 differing pixels.
  - The System window lists its own system in all three lines before any
    drop.
  - The drop on the player's system in uprising logs
    `status=set areas=3 to=Ithor`, and all three lines name Ithor.
  - The drop back on its own planet brings back the first lines.
- **`facility-icon-drop-on-headquarters`**, Alliance only (the Empire sees
  no headquarters in this fixture):
  - The drop on the dome, at the HQ planet's corner plus (18, 18), logs
    `status=refused drop=1`, and the lines do not change.
  - The menu's Destination, clicked at the same point, sets the HQ's
    system in all three lines.
  - The drop on its own planet sets it back.
  - The drop on the planet's edge, at the corner plus (4, 32), sets the
    HQ's system again.

## Verification

- **Gate on Linux.** The gate ran on Chromium 155.0.8059.84, through a
  temporary copy that uses the Linux browser manifest, with `--no-build` on
  the WASM from `scripts/build-interface-test-wasm.sh`.
  - Before the fix (test WASM sha256
    `a57f7b994aba303ab43656eac069f57f9eba0b044ccde6df882196f34b1eda3d`, run
    `sector-quadrants-2026-11-13T22-31-01-688Z-725956`):
    `facility-icon-drop-on-headquarters` failed: the drop on the dome set
    Fwillsving. `facility-icon-drop` passed on both sides.
  - After the fix (test WASM sha256
    `4a365bb534f5f7c04d1acc130efbd37aa3ed982a34f2e42501f70ee8fd802090`,
    run `sector-quadrants-2026-11-13T22-37-45-600Z-763758`): all three
    results pass.
  - The full gate ran once on the same WASM, in run
    `sector-quadrants-2026-11-13T22-38-58-035Z-766196`: 50 of 50 results
    passed.
- **Native tests.**
  - `cargo test -p rebellion-render --lib`: 1592 passed.
  - `cargo test -p rebellion-app --features interface-test-fixtures`: 192
    passed.
- **Mutants.** `cargo mutants --in-place` on `headquarters_at` and
  `destination_drop_target`: 6 mutants, 6 caught. No mutant is generated
  inside the new early return of `destination_drop_target`. The browser
  case covers that return; the unit tests do not.
- **Provenance and formatting.** The provenance scan reports 0 uncited
  items. rustfmt shows no hunks in the touched code; the baseline hunks
  elsewhere remain.

## What stays open

- **Other objects at the point.** Only the HQ overlay is treated as an
  object. A quadrant icon over the planet is still taken to answer its
  system (the hyp in `targeting::mission_release_target`). The other
  objects listed in the notes (a planet's squadron, an unfinished unit, a
  Missions window's mission) still answer their system.
- **No advisor reaction.** The refused drop's advisor reaction
  (`FUN_00487c90`) is not sent for this refusal.
- **Window placement.** The facilities window opened at another place in
  A0 16. That was not studied.
- **Empire side.** The Empire has no A0 for these drops. It ran the
  port-only checks of `facility-icon-drop`.
