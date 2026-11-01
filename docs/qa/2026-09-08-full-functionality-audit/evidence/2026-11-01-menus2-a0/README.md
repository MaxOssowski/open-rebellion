# Object menus, mission, move and Encyclopedia against A0 (2026-11-01)

A0: the Windows 10 VM, a new Alliance game (save "test", days 7-41),
HQ Vjun in the Moddell sector. Captures in `.artifacts/a0-vm/menus2/`,
hashes in `a0-sha256.txt`.

## What A0 shows

- `01`: the fleet icon's menu over the player's own system lists Move,
  Confirmed Move, Planetary Bombardment, Planetary Assault, Encyclopedia,
  Status, Scrap; Assault and Status are grey (OBJ-14-C002, C011).
- `02`: Planetary Bombardment's submenu opens on the parent's left, its
  top three pixels above the row: Target Military Facilities, Target
  Civilian Facilities, General Bombardment, Destroy System (grey). The
  highlighted parent row's text is red (C011, C018).
- `04`: the fleet icon's Encyclopedia opens the system's topic (Vjun):
  an icon's object is its system, as the port's `SystemIcon` (C014).
- `06`, `10`: a character's menu (Move, Confirmed Move, Mission,
  Command, Encyclopedia, Status, Retire); Mon Mothma's Command and Retire
  are grey (C005, C009).
- `08`, `11`, `12`, `13`: Mission shows the targeting cursor; a release
  on the team's own cell or on its own planet in the sector window opens
  Create Mission at that system (Recruitment), and the kind box drops a
  scrolled list. A release on Adega or Basilisk (neutral, other systems)
  opened nothing: no kind was legal there for these characters (C009).
- `15`: the mission icon's menu: Encyclopedia, Status (grey), Abort, the
  day after the mission started (C009).
- `16`-`18`: the fleet icon's Move, the targeting cursor over Adega, and
  the release: the fleet icon moves to Adega at once (C010).
- `19`, `20`: the fleet's own menu in the Fleet window; its Encyclopedia
  opens a "Fleet" topic (EDATA.132, "Fleets are basically groups of ships
  that work or travel together.").
- `21`, `23`, `24`, `25`: from Fleet, next opens Fwillsving and
  previous Flax; Flax's next is Fleet again, and the index lists Fleet
  between Flax and Fwillsving, selected, "Fleet" in the Topic box.
- `05`: the Personnel Finder's right-click menu (Encyclopedia, Status).

## Change: a fleet's Encyclopedia

`FUN_0045d400` looks the object up in the catalog; a fleet has no record,
so it takes the live object's name `+0x30`: text
`(0x2d03 & 0xfff) + 0x1000` (ENCYTEXT 7427), art by the fleet family's
side variant (Alliance EDATA.132, Empire EDATA.133). The port opened the
index instead.

- `rebellion_data::encyclopedia_topics::fleet_topic_binding` builds that
  topic (title TEXTSTRA 11523, key `FLEET_TOPIC_OBJECT_ID` = family 8);
  `installed_encyclopedia_topics` adds it (357 bindings, 347 complete).
- `open_original_encyclopedia_unlisted` opens the window at it and keeps
  it on All Databases for the window's life (`original_extra_entry`,
  `window_entries`); hyp: the other databases leave it out.
- The Encyclopedia order logs
  `command=0x100 destination=encyclopedia status=opened ...`.

Tests: `a_fleet_topic_takes_the_fleet_name_text_and_each_sides_art`,
`the_installed_source_binds_the_topics_when_present` (357/347 with the
fleet), `an_unnamed_objects_topic_joins_the_window_list_by_its_name`.
`fleet-window.mjs fleet-encyclopedia` on Linux Chromium 155, both sides:
the fleet's menu opens topic `0x08000000` ("Fleet"; the test site has no
EDATA, so no picture), next steps to the list's next name and previous
comes back to Fleet. Run `fleet-window-2026-11-01T10-02-49-502Z-98525`, test WASM `6e2562f63134829d5317ecd6478f0ab6ac219ef57740d6f6fbc1ae250c06da95`:

```
8566d4c90a8fb993a3b37baaa8d9dc12d3c439536e9b17e5befeb841015a6cce  alliance-fleet-encyclopedia/fleet-topic-next.png
64528d9fc128a04e5f47aaa6d11354a4640f26b747fe52f63c26f52d19c7eb4d  alliance-fleet-encyclopedia/fleet-topic.png
e63c6dda4e34d90f10e019d9f849ae8280d7e0fbd76cd5e4e4bd24047efccbc2  empire-fleet-encyclopedia/fleet-topic-next.png
eb5e9e96534198562aa42a2c42d7e8642f8be25b3441ab15c7b15855befe56b1  empire-fleet-encyclopedia/fleet-topic.png
```

Mutants: `encyclopedia.rs` 19 caught, 1 unviable, 1 missed:
`window_entries`' `<` for `<=` only orders an extra entry against an
equal name, which no evidence settles. `encyclopedia_topics.rs` 1
caught, 4 unviable.

## The bombardment submenu's frame

Measured on the frames' (192, 192, 192) borders:

| | parent | submenu |
|---|---|---|
| A0 `02` | (266, 136)-(437, 277), 172 by 142 | (120, 175)-(268, 256), 149 by 82 |
| port `fleet-window.mjs bombardment` | (347, 75)-(518, 216), 172 by 142 | (201, 114)-(349, 195), 149 by 82 |

In both the submenu's right edge is two pixels right of the parent's
left edge and its top 39 pixels below the parent's top (the row's top
less three, `submenu_origin`), the rows are the same four with Destroy
System grey, and the highlighted parent's text is red. The port's
capture comes from the batch run of 2026-11-01 (Linux Chromium 155, run
`fleet-window-2026-11-01T09-00-15-435Z-22304`):

```
7a4617df01af5228af0fbb558de894dbf0f211f2160d5f1bd1fa91a6e24f155c  alliance-bombardment/bombardment-submenu.png
```

## The mission kind's name

A0 `11`-`13` draw the shown kind's name white, centered along the box's
top, its capitals from y 103 to 112, over the icon centered in the box.
The box's text color is its `+0x94`, white (`FUN_0060bed0`, set again by
`FUN_0046a9c0`), which `FUN_0060c210` draws the shown item's name in; the
item's `+0x60` gray is not the one shown. The port drew it gray, small and
under the icon (a `hyp:`). It now draws it white in the default font 4
(13 ppem, `paint_gdi_text`), centered from the box's top: the capitals of
"Espionage" span y 103 to 112 as A0's "Recruitment" do.
`the_kind_name_is_white_along_the_box_top` fails without either part.
`mission-dialog.mjs` passes on Linux Chromium 155 (run
`mission-dialog-2026-11-01T10-16-58-353Z-147573`, test WASM `fe29f4ae5e377b14d1ce00a1533a9eec6df899a1190a2f0f049134fb2c6a852f`):

```
4c10372a95f08fd068d762624af1af63e57c4dcefa98f0ba88606de6be493ab9  alliance-targeting/opened-screen.png
```

## The dialog's title, "Target" and target name

`FUN_0046a9c0` sets font 5 (`FUN_00601c60(5)`), bold Arial at 13 ppem,
before the title at (5, 2), "Target" at (37, 195) and the target's name
(37, 294, 185 wide, centered). The port drew them in an 11-pixel regular
face. Now through `paint_gdi_text` in font 5: the title's ink spans x 3
to 95 and y 3 to 12 in the window against A0's 4 to 94 and 3 to 12, and
"Target" x 35 to 74 (A0 the same) and y 197 to 209 (A0 196 to 209).
`the_title_and_target_label_are_in_font_5` fails in font 4.
`mission-dialog.mjs` passes on Linux Chromium 155 (run `mission-dialog-2026-11-01T10-22-23-743Z-152645`); its
"Target" text mask grows one row, to y 209, where font 5's g descends
in A0 too.

```
a28a9df0f1fc149f66819141354051ce8a6463c8c054fffac6fea122d5aa5e36  alliance-targeting/opened-screen.png
80cc7e074e91832f7fbbf46772c4b9b1d99be23bae4d08ab538e8c6934f545b6  empire-targeting/opened-screen.png
```

## The targeting cursor and the dialog against A0, pixel by pixel

- **Cursor.** A0 `08` draws REBEXE cursor group 1002 with its hotspot
  (12, 12) on the pointer: all 89 of its white pixels land where the
  port's resource puts them (offset (140, 92) for the pointer at
  (152, 104)). The port's `mission-dialog.mjs targeting` compares its
  cursor with the same resource exactly.
- **Dialog.** A0 `11` and the port's `targeting` capture, both at
  (170, 40), compared over the 259 by 355 window with the text, the kind
  box, the target art and A0's "Selected Mission" tooltip masked: 850
  pixels differed. Two causes:
  - the title strip: `FUN_0046a9c0` lays the 240-pixel bar twice, 30
    apart, then copies 255 by 16 of it to (2, 2)
    (`FUN_005fcc30(.., 0xff, 0x10)`). The port drew 17 rows and the
    second bar to the window's edge; it now clips the strip to 255 by 16;
  - the close box 10108/10109 and the tabs 11103..11110 are
    CoolStrobeButtons, blitted keyed on their first pixel: A0 shows the
    red bar at the close box's bottom-left pixel and the panel's white
    at each tab's. They are now keyed (`uses_first_pixel_key`).

  After both: **0 of 47,985 compared pixels differ**. The gate's composed
  expectation follows the same rules. Run `mission-dialog-2026-11-01T14-06-01-130Z-819067` (Linux Chromium 155),
  test WASM `f68acbfd4bc656a642635d5ffeff848574057709a615f039e6019898bf19d8fb`:

  ```
2e83d04e4e76931787d12c2c5f79d6f75e3f5b91f2e5bc6aa6b12b6ff50ac5d2  alliance-agents/opened-screen.png
51719ebec13abcef0c293330d4782f139c50fa7617bf2f57a7d833cab4df4b28  alliance-mission/opened-screen.png
c910db1d2b496224cdeb3ae0bf8b8b2b03f9834dd28f2e2018a1efc3feb43124  alliance-targeting/opened-screen.png
8191b9e4b26b4fef2df42058eb2d6d6782dc569e843c0f1ecf0bc6b4a699b149  empire-agents/opened-screen.png
28fbbc587e0eea6378f13ccccb2aae60918271104e26dc81166e9a86581af327  empire-mission/opened-screen.png
45d376d7627ce4265538d9e7e24cb1ef2b70988f09b28bcba6fb1bb39318bcb0  empire-targeting/opened-screen.png
  ```

## Open

- Which list item an arrow from Fleet takes in the port's galaxy differs
  from A0's only by the galaxy's systems (A0's list has no Fornax, since
  Fleet's next there is Fwillsving).
- Planetary Assault and Destroy System still have no A0 capture of an
  enabled order (C012, C013).
