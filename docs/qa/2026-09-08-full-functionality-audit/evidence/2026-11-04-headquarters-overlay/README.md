# The headquarters overlay: its press, menu and Status (2026-11-04)

This covers surface ledger cells OBJ-13-C003 ("HQ status") and OBJ-13-C004 ("HQ item menu"). The reading is in `ghidra/notes/headquarters-overlay.md`.

## The original

- **The overlay.** The sector window's headquarters mark 904 is an overlay (id `0x10000`, `FUN_0045bbb0`), and its object is the HQ (ALLFACSD, family `0x20`).
- **Order in the list.** The overlay list is sorted by id (`FUN_0060a790(1)`, `Class_0066e148_vt4`), so the HQ overlay comes before every quadrant icon.
- **Hit test.** `FUN_0045cc10` gives a press to it wherever 904 is opaque (`FUN_005fca00`, offsets above 0).
- **Menu.** The HQ's slot `+0x3c` (`FUN_00526860` → `FUN_00526c00`) lists Move, Confirmed Move and `0x204`, which has no row. `FUN_0051d990` adds Encyclopedia and Status, with Status enabled. There is no Scrap. The manual (p. 134, Fig. 3.82) agrees: "right-click, select Move".
- **Status.** Families `0x20..0x27` take `FUN_00444e20`, Defense Facility Status: Location, Status, Maintenance Cost and Bombardment Defense Strength (class `+0x58`, ALLFACSD's 9). There are no weapons or shield rows.
- **Quadrant icons.** The HQ lights neither the facilities icon (`FUN_0053b6e0`, `0x28..0x30`) nor the defenses icon (`FUN_00526fd0`, `0x22..0x28`).

## Before

- The port drew 904 but took no press on it. A right click there opened the quadrant icon's menu, or nothing.
- The HQ's Status read "Manufacturing Status", with a processing rate.
- An HQ counted toward the facilities icon.
- A dead egui side panel (`draw_system_info_panel`, its "HEADQUARTERS" label among them) was still in `lib.rs`. Nothing called it, and it is removed.

## Now

- **The overlay.** `sector_window.rs` tests it first. A press selects it, and the right release opens `MenuObject::Headquarters`. A double click on it opens no icon's window.
- **The menu.** It reads Move, Confirmed Move, Encyclopedia, Status. At first Move and Confirmed Move were grey, because the HQ did not move. Since the later 2026-11-04 batch they are white, as in A0 (`../2026-11-04-headquarters-move/`).
- **Status.** It shows Defense Facility Status with the four rows.
- **Quadrant icons.** The facilities icon leaves the HQ out.
- **The fixture.** The HQ object now moves with the `is_headquarters` flag. The mark stands for that object.
- **The `system-menu` gate.** It now right-clicks the uprising system's planet. The headquarters planet's centre is opaque on 904, so it is not an empty planet.

## Tests

Unit tests (each fails without its change):
- `the_headquarters_shows_defense_facility_status_with_its_bombardment_defense`. Without the change the title is "Manufacturing Status".
- `the_headquarters_lists_move_confirmed_move_encyclopedia_and_status_without_scrap`.
- `the_headquarters_takes_a_press_on_its_opaque_pixels_before_the_icons`. Without the change the fleet icon takes the press. The test checks:
  - an opaque pixel under the fleet icon;
  - a keyed pixel;
  - the x = 0 column;
  - a double click;
  - a later press on the icon.
- `the_headquarters_lights_neither_the_system_nor_the_defenses_icon`.

`sector-quadrants.mjs` `headquarters-menu` (Linux Chromium):
- **Alliance.** A right click on 904's (20, 12) lists Move and Confirmed Move grey, and Encyclopedia and Status white. Status then opens "Defense Facility Status" with Location, Status "Active", Maintenance Cost and Bombardment Defense Strength "9".
- **Empire.** It does not see the HQ, so the press selects nothing.

Run `sector-quadrants-2026-11-04T11-08-21-918Z-1074795` (`icons`, `system-menu` and `headquarters-menu`, both sides): pass.

## A0

Captured in the Windows 10 VM: an Alliance save, day 488, the Moddell sector window. The HQ stands at Pzob. The files are in `.artifacts/a0-vm/headquarters/`.
- `00-moddell`: 904 over Pzob.
- `01-hq-menu`: a right click at (148, 272), on 904's opaque pixels, opens a 104 by 82 menu at the cursor. Its rows are Move, Confirmed Move, Encyclopedia and Status, all white. The port's menu matches it in size, place and rows. Its Move and Confirmed Move were grey at first, and are white since the HQ moves (above).
  - While the HQ is selected, the part of Pzob the menu leaves uncovered is pixel-identical to `00-moddell`. So the selected overlay keeps 904, as the port draws it.
- `02-hq-status`: "Defense Facility Status" at (108, 79). Its rows are:
  - Location Pzob;
  - Status Active;
  - Maintenance Cost 0;
  - Bombardment Defense Strength 9, on two lines;
  - with picture "Alliance Headquarters".

  Against the port's window at the same place:
  - the picture frame and the buttons are exact, 0 pixels differing;
  - the title, the five label lines, the name and three values each ink within 1 pixel.

  `headquarters-menu` asserts all of this when the capture is present.

## Open

- Moving the HQ is covered in `../2026-11-04-headquarters-move/`.
