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

## Open

- Which list item an arrow from Fleet takes in the port's galaxy differs
  from A0's only by the galaxy's systems (A0's list has no Fornax, since
  Fleet's next there is Fwillsving).
- Planetary Assault and Destroy System still have no A0 capture of an
  enabled order (C012, C013).
