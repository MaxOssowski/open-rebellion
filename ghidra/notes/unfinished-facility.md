# An unfinished facility's menu, Status and picture (read 2026-11-04)

A facility ordered from a construction yard is an object from the start.
It stands in its destination's facility page, not yet completed (`+0x50`
bit 2 clear), or en route there once built (bit 4). `FUN_00458fe0` draws
its page art: base + 2 (side 1) or + 1 under construction, base + 3 en
route (`manufacturing-build-selection.md`).

A0, the Windows 10 VM, Alliance, day 563 (`.artifacts/a0-vm/pending-facility/`):
- **The setup.** Pzob's construction yard was building a refinery for
  Commenor ("Building: 1, Destination: Commenor", `05-system`).
- **The pages.** Pzob's Refineries page was empty (`06-tab*`).
  Commenor's showed two refineries under construction in STRATEGY 9032,
  each 0 of 2145 pixels off (`11-commenor-refineries`).
- **The menu.** A right click on one opened Encyclopedia, Status and a grey
  Scrap (`12-menu`).
- **Status.** "Manufacturing Status" (`13-status`), with these rows:
  - Location: Commenor;
  - Status: Under Construction;
  - Maintenance Cost: 0;
  - Standard Processing Rate: 5;
  - Bombardment Value: 2.

  The picture is the refinery over a red grid.

## The Status

- **Rows.** `FUN_00446c50` writes Location (`0x8664`). When the object is en
  route, `FUN_004fd2b0`'s rows follow: 34662 "ETA Destination:", "Day ".
  Then it writes Status (`FUN_0044a470`):
  - 34628 "Enroute" for bit 4;
  - 34659 "Active" for bit 2;
  - 34647 "Under Construction" otherwise.

  Maintenance Cost (`FUN_0044a620`), Standard Processing Rate and
  Bombardment Value follow.
- **Order against the Defense Facility Status.** `FUN_00444e20` puts the
  ETA before Location instead.
- **The picture.** `FUN_00443130` draws it with `FUN_0042c3b0(gokres, id,
  object, 1, 1)`. The marks go into the picture's bitmap behind the
  object, and the window blits the result keyed (`FUN_005fd0f0`):
  - en route (bit 4, not destroyed): STRATEGY 11510 for families
    `0x22..0x27`, and 11506 for the other `0x20..0x2f`;
  - not completed (bit 2 clear, `0x42c... & 4`): STRATEGY `0x2d32 + 1`
    (11571) for side 1, and `0x2d34 + 1` (11573) otherwise. The minis take
    11570 and 11572.

  Each mark has one blue key pixel. 11571's is at (0, 48), so the frame's
  black shows through it in A0.

## Ported (2026-11-04)

- **The page item.** `FacilityItem::Pending` carries the unit's class, side
  and arrival day. Its menu object is `MenuObject::PendingFacility`. That
  takes the facility list (Scrap), with no scrap target, so Scrap is grey.
- **Status.** `StatusObject::PendingFacility` gives the Manufacturing Status
  above, with mark 11571, 11573 or 11506. It closes once the unit leaves the
  queue or arrives.
- **The HQ en route.** It now takes 11506 too.
- **Keys.** `bmp_cache` keys the marks 11505/11506, 11509/11510 and
  11570..11573.
- **port:** an unfinished defense facility (`0x22..0x27`) is not on the
  facility pages, and its Defenses window row is not ported.
