# Navigation: the System window's facility page and the uncommon states, settled (P06, CMD-03)

Date: 2026-10-28. Docs only.

## The facility page against A0

A0, Windows 10 VM, original `REBEXE.EXE`, the "test" save, day 8: Vjun's
System window on its Construction Yards tab
(`.artifacts/a0-vm/2026-10-28-nav/system-vjun-yards.png`, SHA-256
`60028c08e49c9b492c9d53dee56ca894a759c78e70233dbdc289390ab29485c8`).

- The window's title strip starts at (229, 88), so its origin is (227, 86).
- Matching each facility picture (STRATEGY 9001..9033, matte keyed) against
  the capture puts 9022 at (236, 165), with 0 of 223 sampled pixels
  differing. That is (9, 79) in the window: the list's (8, 77) plus the
  picture's (1, 2) offset.
- The port maps a construction yard to 9022 (`facility_picture`) and draws
  it at (9, 79) (`a_facility_page_shows_its_label_pictures_and_selected_frame`).
- The page label "Construction Yards" is white and centred above the
  list, in font 4 since `../2026-10-28-facility-menus/`.

## The uncommon states

| State | Sector window | Settled by |
|---|---|---|
| unexplored | no tracks, no icons, white name | A0 Moddell + `FUN_0045c240` (`../2026-10-28-sector-track-visibility/`) |
| uninhabited | no support track, white name | `FUN_0045baf0`; code only |
| neutral, explored | white name, all tracks | A0 Sesswenna (`../2026-10-28-sector-names-and-marks/`) |
| side 1 or 2 | red or green name | A0 Sesswenna, Moddell |
| headquarters | mark 904 over the planet | A0 Moddell (Vjun) |
| uprising | mark 905 or 906 in the planet | `FUN_0045bbb0`; code only |
| destroyed | asteroid field 10240, cyan name, no tracks, no marker | `FUN_0045bbb0`; code only |
| blockaded | nothing in the sector window | No sector-window function reads `+0x88` bit `0x20`. The GID's blockade mode shows it (GID gate). |

## What stays open in P06

- A0 for the uprising and destroyed states. The VM has no such system at
  hand, and making one needs a long campaign or a Death Star.
- The menu of a pending unit and of an empty deposit on a facility page.
  Their objects are untraced, and the port's pending items are queue
  entries, not objects.
