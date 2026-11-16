# EVT-02-C008: the Battle at Yavin alert with Retreat disabled (2026-11-16)

A0 2026-11-16-partial-cells/v-14a-battle-at-yavin-alert-retreat-grey-d977
(sha256 `b0ce92ad9065000b81a694458cf71af4c84f108393376c1f116641ad92d69356`)
is a new Alliance game's Battle at Yavin alert on day 977. The README says
the alert came twice with Retreat grey, and the following Take Command
(v-14, `dc15d6f2f4ee0f30ff4fdf8906e66ed171a18bbfb1a3d4aa085888b640473226`)
made REBEXE exit to the desktop.

## What the capture shows

- **Title and text:** "Battle at Yavin" over "The Alliance fleet has entered
  the Yavin system. Imperial forces have been detected on an intercept
  course."
- **Retreat is the disabled art.** STRATEGY 10976
  (`BATTLE_ALERT_RETREAT_ALLIANCE_DISABLED`), 134 by 27, matches the capture
  at (74, 345) in every opaque pixel. The 40 differing pixels are all the
  bitmap's blue key (0, 0, 255) on its top row. The other alert button
  bitmaps 10971..10975 differ at best by 205 pixels (10973, the Empire's
  disabled Retreat) and otherwise by 2645 or more.
- **No forces list.** The capture is the alert's first page. Neither the
  Alliance's nor the Empire's forces list was opened, so neither shows. The
  picture is the alert's scene and does not show which ships are at Yavin.

## Verdict

The capture shows that Retreat was disabled. It does not show why. A
gravity-well ship (an Interdictor Cruiser, CAPSHPSD 130, or a CC-7700
Frigate, class 74, both with `gravity_well_projector` 4) cannot be seen or
ruled out.

`FUN_00516730`'s other two conditions cannot be read from it either: a
movable unit of the viewer's side, and a destination the autoroute search
accepts (2026-11-12-evt-02-c008-cause). The text names an Alliance fleet
entering Yavin, so the viewer had a fleet in the battle. The trace says that
fleet always gets a candidate destination unless the unit is refused (other
side, unfinished, destroyed or en route).

The capture does not contradict the port's rule
(`abstract_battle::retreat_offered`), so the port is not changed. This is a
second A0 case of the disabled art, on the Alliance side (after
battle-clean/20, Umgul). The cell stays partial.

## What would settle it

Before choosing Retreat, Take Command or Simulate, open both forces lists of
such an alert and scroll each to its end. An enemy Interdictor or CC-7700
compares the gravity-well output directly.
