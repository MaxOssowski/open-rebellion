# Travelling members at their destination carry the en route mark

User report (2026-11-12): units on the move should already show at their
destination, drawn on the hyperspace (en route) background, as in the
original.

## What the port did

Checked per kind before the change:

- Fleets: listed in the destination's Fleet window with the en route
  entry and picture (fleet-window.mjs a0-destination, A0 fleet4).
- Regiments moving to a planet: listed on its Trooper Regiments page over
  their en route mark (A0 2026-11-11-regiment-en-route/00, commit ad1bf1d7).
- Characters and special forces on a Move: listed on the destination's
  Defenses personnel page (commit 9e7e93c0), but on the plain backdrop
  STRATEGY 11500. This was the gap.

## Original behaviour

`FUN_0042c3b0` (decompiled-typed, line 187): when the object's `+0x50`
has bit `0x10` (in transit) and not bit 8, and the caller asks for marks,
it draws the en route mark under the mini: STRATEGY `0x2ced` (11501) for
personnel, the class's own GOKRES mark (`(class & 0xfff) + 0x5000`, the
mini + 0x1000) for special forces `0x3c000003` and `0x3c000005`.

STRATEGY 11501 is the grey bulkhead with hyperspace streaks through its
ports; 11500 is the plain radial backdrop.

A0 evidence:

| Capture | SHA-256 | Shows |
| --- | --- | --- |
| `.artifacts/a0-vm/specforces/03-mon-calamari-personnel.png` | `bdf0a519cd741cecc58e93c53610e0fb3a672e96158e81513e2b666d8a3add3f` | Garm Bel Iblis and Adar Tallon, travelling, on 11501; Syub Snunb on 11500; the Longprobe team on GOKRES 21826 (17730 + 0x1000) |
| `.artifacts/a0-vm/sfstatus/01-yavin.png` | `118759db928ce5700be1348362a359e2ba5a71de332b3925dcd85e3a882aeb23` | The Infiltrators travelling to Yavin, third cell, mini at (452, 167) on 11501 |

The OBJ-10-C004 note of 2026-11-03 read the Longprobe team as "no mark";
that comparison covered only the mini's own pixels, where the mark does not
show.

## Change

`defenses_window::row_marks` takes the mission transits
(`MissionState::en_route`) and marks a listed character or special force
en route while it has one. The marks come from the existing
`fleet_window::mini_marks`/`en_route_mark`.

## Verification

- Unit test
  `members_on_their_way_take_the_en_route_mark_over_their_backdrop`
  (fails without the change: `[(Strategy, 11500)]` against
  `[(Strategy, 11500), (Strategy, 11501)]`). The regiment test now also
  checks that the planet's own regiments carry no mark.
- `cargo mutants -p rebellion-render --file
  crates/rebellion-render/src/defenses_window.rs --re row_marks`: 12
  mutants, 10 caught, 2 unviable.
- Browser, Linux Chromium (`.browser-linux.json`), `status-window.mjs`
  case `special-force-en-route` (fixture SpecialForceEnroute, code 104):
  the Infiltrators' cell found in the port's Defenses window and compared
  with A0 sfstatus/01 (452, 167), 61 by 25, every pixel but 11501's key
  pixel (0, 24): 0 of 1,524 differ. Run
  `status-window-2026-11-12T18-09-01-844Z-2708331`, 26/26 cases pass,
  test WASM `c996e270753af5982cec02badda75675e01de7c7ed96e5f8915db3635d0e99b4`.
  The same gate against a WASM built without the change
  (`70045fe246797a6ad545e733477fe387833764b7ebdb9479d4d6c1d878bcf5a6`,
  run `status-window-2026-11-12T18-15-18-195Z-2727627`) fails: 1,263
  pixels differ.

## Open

- Squadrons sent by Move (`SquadronTransit`) are listed nowhere while they
  travel; the original puts them in the destination container at once
  (`FUN_00556430`). The Defenses window's squadron page and the Fleet
  window's squadron list would need a travelling row with its own menu and
  Status. No A0 capture of a travelling squadron.
- Regiments moving into a fleet (`RegimentLeg::Fleet`), squadrons moving
  into a fleet, and members whose Move targets a fleet are not listed in
  that fleet's window while they travel (members show on the system's
  Defenses page instead). No A0 capture.
- The Defenses quadrant icon does not count regiments or squadrons
  travelling to an otherwise empty planet.
