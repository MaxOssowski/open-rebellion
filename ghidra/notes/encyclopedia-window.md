# Encyclopedia window construction

This note separates the original Galactic Encyclopedia from the Message Index.
The two surfaces were conflated in P63 because both load STRATEGY resources and
the Message Index constructor also opens `encybmap.dll`. The executable and
original screenshots establish distinct routes, layouts, and control sets.

## Entry and dimensions

`FUN_00429f30` is the command-center Encyclopedia route. It checks window ID
`0x19`, then constructs `FUN_0045d400` at 470 by 330. The constructor installs
vtable `PTR_FUN_00659fa8`, loads both `encytext.dll` and `encybmap.dll`, and
resolves an initial game object or falls back to index mode.

`FUN_0045ddc0` is the Encyclopedia layout routine. The client is 470 by 330,
while its faction base resources are 470 by 331 and therefore lose their
bottom row at the client boundary. Index mode composes STRATEGY 10338 at
`(12,13)`, faction base 10335 or 10336, faction rail 10585 or 10589 at
`(412,0)`, and TEXTSTRA `0x1843` at `(36,48)`. Topic mode substitutes STRATEGY
10337 at `(12,14)`. It is not `FUN_004665f0`; that function belongs to the
Message Index.

## Exact index controls

The category parent is `(36,78,361,41)`. Every child has a 49 by 41 control
rectangle. The table is ordered spatially; the constructor emits the controls
in a different order.

| Command | Absolute x | Filter | Label | Alliance normal/pressed | Empire normal/pressed |
| --- | ---: | --- | ---: | --- | --- |
| `0x6f` | 36 | all objects | `0x1850` | 10340 / 10339 | 10340 / 10339 |
| `0x70` | 88 | family `[0x90,0x98)` | `0x1855` | 10350 / 10349 | 10350 / 10349 |
| `0x71` | 140 | family `[0x14,0x20)` | `0x1854` | 10348 / 10347 | 10360 / 10359 |
| `0x72` | 192 | family `[0x20,0x30)` | `0x1852` | 10344 / 10343 | 10356 / 10355 |
| `0x73` | 244 | family `[0x40,0x80)` | `0x1851` | 11616 / 11615 | 11618 / 11617 |
| `0x74` | 296 | family `[0x10,0x14)` | `0x1856` | 10352 / 10351 | 10362 / 10361 |
| `0x75` | 348 | family `[0x30,0x40)` | `0x1853` | 10346 / 10345 | 10358 / 10357 |

The `0x75` resources are 49 or 50 by 57 pixels. The original paints from
source origin `(0,0)` into the fixed control, clipping the bottom 16 rows and,
for the 50-pixel-wide resources, the rightmost column. It does not scale or
center them.

The right rail uses close `0xfb`, topic `0x67`, and index `0x68`. Alliance
rectangles are `(423,25,32,31)`, `(423,93,32,31)`, and `(423,147,32,31)` with
resource pairs 10370/10371, 10374/10375, and 10372/10373. Empire rectangles are
`(426,21,44,41)`, `(426,89,44,41)`, and `(426,143,44,41)` with pairs
10376/10377, 10380/10381, and 10378/10379. Index mode makes `0x68` current and
topic mode makes `0x67` current.

## Text, list, and topic geometry

- main heading or topic title: `(36,14)`, width 350;
- current-object selector: `(143,45,245,18)` in index mode;
- selected-category label: `(40,119)`, width 283;
- object list: `(36,137,350,160)`, command `0x65`;
- EDATA image: native 400 by 200 at `(12,31)` in topic mode;
- topic body: `(17,231,395,80)`;
- previous `0x83`: `(28,14,21,17)`, resources 10385/10386/10387;
- next `0x84`: `(380,14,21,17)`, resources 10382/10383/10384.

Index Left and Right cycle visible category children and wrap. Topic Left and
Right invoke previous and next. Exact localized category names beyond the
source-backed labels must come from the original string resources, not icon
interpretation.

## Source catalog and labels

P65 binds the recovered family ranges to the immutable source DAT catalog.
The owned English TEXTSTRA table resolves `0x1842` to `Galactic Encyclopedia`,
`0x1843` to `Topic`, and the category resources to `All Databases`, `System
Database`, `Ship Database`, `Facilities Database`, `Missions Database`, `Troop
Database`, and `Personnel Database` in command order `0x6f..0x75`.

The supported source families produce 356 entries: 200 systems; 30 capital
ships and eight fighters; six defenses, six manufacturing facilities, and two
production facilities; 25 mission types; ten troop types; and six major
characters, 54 minor characters, and nine special-forces types. A compound
object ID uses the DAT family as its high byte and the source record ID as its
low word. P65 sorts case-insensitively by source name with that identity as a
stable tie breaker, matching the alphabetical index behavior visible in the
classified original capture.

## Recovered workflow

- Index mode uses category commands `0x6f` through `0x75`.
- `FUN_0045f100` rebuilds the object list for a selected category from the
  authoritative game-object collection and updates the selected category label.
- `FUN_0045f480` switches between index and topic modes.
- `FUN_0045fa60` resolves the selected object's EDATA bitmap, loads its text
  resource from `encytext.dll`, updates the topic title, and configures previous
  and next availability through `FUN_0045fd20`.
- `FUN_0045fe60` routes keyboard navigation; in topic mode Left and Right invoke
  commands `0x83` and `0x84`.

## Topic mode (ported 2026-10-23, not routed)

- `FUN_0045fd20` enables previous (`+0x144`, `0x83`) and next (`+0x140`,
  `0x84`) from the selected list item's previous (slot `+0x10`) and next
  (`+0xc`) items, so they step through the current category's list and
  stop at its ends (no wrap).
- `encyclopedia::draw_encyclopedia_topic` paints the topic geometry above
  (base, rail with `0x67` current, STRATEGY 10337 at `(12,14)`, EDATA at
  `(12,31)`, title, `0x83` / `0x84` with their third resource when
  unavailable, body in `(17,231,395,80)`); `apply_original_topic_command`
  and `original_topic_neighbours` carry the state; the index's rail `0x67`
  switches to the topic of the selected object. The topic data comes from
  `rebellion_data::encyclopedia_topics` (P66A) as
  `OriginalEncyclopediaTopic`.
- The body's font and line spacing are the port's (the original's
  `FUN_005f35b0` text object is not read).

## Production routing (2026-10-23)

- `FUN_00429f30` (command `0x131`, the cockpit button and F7) builds the
  window with `FUN_0045d400(parent, 0, 0, 0x1d6, 0x14a, ..)`: 470 x 330
  at `(0, 0)` of its parent. Settled 2026-10-29: the layout routine
  `FUN_0045ddc0` then calls `FUN_00606980` with the galaxy view's
  rectangle (`+0xcc..+0xd8`), so the window is centred in the galaxy view
  with truncating halves: the Empire's at (125, 52) (A0, Windows 10 VM,
  day 143, exact over the frame), the Alliance's at (62, 50) (captures
  kritikanstvo 281117 and 281123). The port's centring in the 640 x 480
  canvas was wrong.
- The app opens the original index / topic whenever the index catalog and
  the side's topics loaded (`rebellion_data::encyclopedia_topics::
  installed_encyclopedia_topics`: native `data/base/encyclopedia/source.json`
  and its manifest; the browser's runtime pack carries them byte for byte
  as `encyclopedia-source.json` / `.manifest.json`). Else the replacement
  stays. The rail's close `0xfb` closes it.
- The cockpit button and F7 (`0x131`) only logged
  `pending_original_window` until 2026-10-23; they open the original
  window when its source loaded. Every opening (cockpit, F7, object menu,
  mission dialog, build selection, status window) shows the index at the
  last selection. Since the same day the object menu's Encyclopedia
  opens at the object's topic (`FUN_0045d400`'s initial object:
  `original_encyclopedia_object`, the class's or the object's DAT id;
  All Databases), or the index when the catalog lacks it; a system icon
  opens its system (port), a fleet or production manager the index. The
  replacement's "focus system" action has no original counterpart and is
  gone with it.


## The index list (read 2026-10-29)

- `FUN_0045ddc0` makes the list with `FUN_00607ea0(.., 0x24, 0x89, 0x15e,
  0xa0, .., 0x65, 0x15e, 0x14, 0x287f, 0x2880, 0x287d, 0x287e, 10, 1, 0)`:
  at (36, 137), 350 by 160, cells 350 by 20, so eight rows; the scroll
  arrows 10367/10368 and 10365/10366, font 10. It sets `+0xe4 = 0x24`
  (`DT_VCENTER | DT_SINGLELINE`) and `+0xe8 = 5`: the text at x 5,
  centred on its row. The list keeps the constructor's colours, `+0xd8 =
  0x787878` (grey) and `+0xdc = 0xffffff` for the selected row; nothing
  else marks the selection.
- Each opening builds a new window, whose list starts with
  `FUN_0045f100(0x6f, 1)`: All Databases, `+0x148` (the selection)
  cleared, so nothing is selected and the selector is empty until a click.
- The labels: the title TEXTSTRA `0x1842` in font 5, centred
  (`format 1`) in (36, 14, 350); "Topic" `0x1843` in font 4, drawn into
  the base at (36, 48); the category label in font 5 at (40, 119), 283
  wide. The selector `FUN_00604cf0` at (143, 45), 245 by 18, takes font
  id 100 (`+0x24`), not mapped.
- The objects come from a list the galaxy view builds once
  (`FUN_00422620`, `+0x474`): every class record of the families
  `0x08..0x0f`, `0x10..0x13`, `0x14..0x1f`, `0x22..0x27`, `0x28..0x2f`,
  `0x30..0x3f` and the missions (`0x50..0x7f`) whose record `+0x5c` is 0,
  then the systems of the viewer's side (`FUN_004f31b0`,
  `FUN_0053ef50(0x90..0x98, side, 1)`). A smaller galaxy's index so
  leaves its absent systems out: the A0 game's index has no Abregado or
  Adrift.
- A0 (2026-10-29, Empire): the rows A-wing, Abduction, Ackbar, Adar
  Tallon, Adega, Advanced Construction Yard, Advanced Shipyard, Advanced
  Training Facility, grey, 20 apart; a click on Abduction turns it white
  and fills the selector.

Port (2026-10-29): `encyclopedia_origin` places the window;
`ORIGINAL_INDEX_ROW_HEIGHT` 20, eight rows, the text at x 5 centred, grey
or white; the fonts above; no first-row selection
(`reconcile_original_index_state`); `open_original_encyclopedia_at`
resets to All Databases; `OriginalEncyclopediaCatalog::set_galaxy_systems`
limits the systems to the world's. Since 2026-10-29 (later the same day)
the list scrolls by pixels with its bar, `FUN_0060a490(.., list, 7,
0x299d, ..)` (RCDATA 10653, the Battle Results' art), arrow step 10; the
scroll returns to the top when the category changes (`FUN_0045f100`).
It matches the A0 bar to 2 pixels. port: the wheel also scrolls, a row
a notch.

## Evidence boundary

- Static source: `FUN_00429f30.c`, `FUN_0045d400.c`, `FUN_0045ddc0.c`,
  `FUN_0045f100.c`, `FUN_0045f480.c`, `FUN_0045fa60.c`, `FUN_0045fd20.c`, and
  `FUN_0045fe60.c` in this directory.
- Visual corroboration: the Encyclopedia index and topic captures classified in
  `docs/qa/2026-09-10-interface-parity-audit/screenshot-ledger.md`.
- P62 proves transport for all 187 owned EDATA images. It does not prove this
  window's category ordering, entity bindings, text, navigation, geometry, or
  A0 parity.
- P65 proves the 356-entry English source catalog, source-family filtering,
  alphabetical ordering, stable row identity, and scrolling inside both
  faction shells. The route remains test-only.
- P66A proves strict local English ENCYTEXT/ENCYBMAP extraction plus ordinary,
  system-picture, and factional artwork binding for 346 topics per faction.
  Browser transport and visible topic composition remain open.
- No `OBJ-01` cell is accepted. Other localizations, exact font rendering,
  browser/runtime source ingestion, topic composition, navigation, contextual
  entry, visible missing entries, production routing, and original-runtime
  comparison remain open.
