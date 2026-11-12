# Encyclopedia: is "Fleet" in every index?

Date: 2026-11-12. Cell: OBJ-01 (Encyclopedia index contents). This is a
static trace only. No code, WASM, gate or VM run. It does not edit the
ledger.

## What was asked

`2026-11-12-encyclopedia-missing-entry/README.md` found that the
original shows a topic only for a key in the galaxy view's list `+0x474`
(`FUN_0045d400`, `FUN_0060a860`). The original shows a "Fleet" topic
(key `0x1d03`, A0 `menus2/20`). So "Fleet" may be in every index. The
port lists it only after a fleet's Encyclopedia was opened. How is
`+0x474` filled? Is Fleet, or any other generic topic, always listed?
In which category and order?

Sources: `~/Games/rebexe-decompile/decompiled-typed/FUN_<addr>.c`,
`ghidra/notes/encyclopedia-window.md`, `fleet-names.md`,
`move-order.md`, the shipped `FLEETSD.DAT` and `MISSNSD.DAT` (read with
`dat-dumper`).

## The trace

### Who builds `+0x474`

`FUN_00422620` is the only writer. Its callers in the corpus are
`FUN_00415420` and `FUN_00440f80`. It fills the list once: the block runs
only while `+0x474 == 0`. So the list does not change during a game.

### What goes in

The first loop walks every class record in the class registry
(`FUN_0051caf0` → `FUN_00567800` over `DAT_006bb7a4`). It keeps a record
whose family byte (`+0x24 >> 24`) is in one of:

- `0x14..0x1f`, `0x28..0x2f`, `0x22..0x27`, `0x10..0x13`, `0x30..0x3f`,
  `0x08..0x0f`;
- `0x50..0x7f`, but only when the record's `+0x5c` is 0.

For a kept record it takes the name `+0x30`, makes the key
`(name & 0xfff) + 0x1000`, and adds an item only if `FUN_0060a860` does
not find that key yet. The item's text is the name string
(`FUN_005f3010`, fallback `+0x34`). The item stores `family << 24`
(`local_78`), which the categories filter on.

The second loop adds the systems of the viewer's side (`FUN_004f31b0`:
`FUN_0053ef50(0x90..0x98, side, 1)`), keyed the same way.

### The family `0x08..0x0f` records

`move-order.md`: the family range `0x08..0x0f` is `FLEETSD.DAT`. The
shipped file has two records:

| Record | Family | Name | Key |
| --- | --- | --- | --- |
| 4 (`0x08000004`) | `0x08` | TEXTSTRA 11523 (`0x2d03`), "Fleet" | `0x1d03` |
| 5 | 0 | 0 | none (family 0 is in no range) |

`0x08000004` is the class every fleet is made from (`FUN_0051af00`,
`FUN_0051b080`, `FUN_0051b1f0` and `FUN_00512d00` pass `0x8000004` to
`FUN_004f7d50`). Its name is the "Fleet" that `fleet-names.md` gives
each new fleet (`FUN_004f9190`).

So `FUN_00422620` adds "Fleet", key `0x1d03`, to `+0x474` at the first
build. It is in every Encyclopedia index of the game, with or without a
fleet. It is the item that `FUN_0045d400` finds for a fleet's
Encyclopedia (the fleet's record name `0x2d03`).

### Other generic topics

None. The other kept ranges are the object DATs the port already reads:
TROOPSD (`0x10..0x13`), CAPSHPSD and FIGHTSD (`0x14..0x1f`), DEFFACSD
(`0x22..0x27`), MANFACSD and PROFACSD (`0x28..0x2f`), MJCHARSD, MNCHARSD
and SPECFCSD (`0x30..0x3f`), and MISSNSD. Not kept: ALLFACSD (`0x20`),
SECTORSD (`0x80..`), ABODESD (`0x98`), MANMGRSD (`0xa0..`), BASICSD and
UNIQUESD (`0xf0..`).

MISSNSD: the families `0x41..0x44` fall outside `0x50..0x7f`. All four
records are also hidden (`+0x5c` = 1). So the port's `hidden_missions`
filter gives the same list.

Not checked: whether two kept records share a key, which
`FUN_0060a860` would list once. The port pushes every record.

### Category and order

`FUN_0045f100`: All Databases (`0x6f`) shows `+0x474` whole
(`FUN_00608280`). The six other categories filter by family:
`0x90..0x98`, `0x14..0x20`, `0x20..0x30`, `0x40..0x80`, `0x10..0x14`,
`0x30..0x40`. None covers `0x08`. So Fleet is only in All Databases.

Order: alphabetical, as the rest of the list. A0 `menus2/25` lists Fleet
between Flax and Fwillsving. Its place does not depend on how the window
was opened.

## The port

- `load_encyclopedia_catalog`
  (`crates/rebellion-data/src/encyclopedia_catalog.rs`) reads SYSTEMSD,
  CAPSHPSD, FIGHTSD, DEFFACSD, MANFACSD, PROFACSD, MISSNSD, TROOPSD,
  MJCHARSD, MNCHARSD and SPECFCSD. It does not read FLEETSD. So no
  catalog entry is "Fleet".
- `fleet_topic_binding` (`crates/rebellion-data/src/encyclopedia_topics.rs`)
  binds the topic to a made-up id, `FLEET_TOPIC_OBJECT_ID = 0x0800_0000`
  (`port:` tag).
- The object menu's Encyclopedia on a fleet (`crates/rebellion-app/src/main.rs`
  ~6832) calls `open_original_encyclopedia_unlisted`
  (`crates/rebellion-render/src/encyclopedia.rs` ~1227). It sets
  `original_extra_entry`. `window_entries` (~1246) puts that row into All
  Databases in its alphabetical place, only while that window lives.
- A window opened any other way (F7, galaxy command `0x76`, Mission
  dialog, Status window, Message Index) has no Fleet row. The original
  has one.
- The category side is right: `window_entries` adds the row only to
  `0x6f`, and the catalog's ranges match `FUN_0045f100`.

## Verdict

**Port bug.** The original lists "Fleet" (`0x08000004`, TEXTSTRA 11523,
key `0x1d03`) in All Databases of every index, in alphabetical order. It
comes from FLEETSD through the `0x08..0x0f` range of `FUN_00422620`. The
port lists it only in a window opened from a fleet. No other generic
topic exists.

Fix (not implemented):

1. In `load_encyclopedia_catalog`, read `FLEETSD.DAT` and push each
   record whose family is in `0x08..0x10` (only record 4, "Fleet") with
   `push_entry`. Cite `FUN_00422620`.
2. Use its compound id `0x0800_0004` for the fleet topic. Set
   `FLEET_TOPIC_OBJECT_ID` to it and bind the topic through the normal
   `bind_encyclopedia_topics` path; `fleet_topic_binding` can go. The art
   rule for family `0x08` (Empire `+0x2000`) already sits in
   `object_artwork_lookup_key`.
3. The fleet's Encyclopedia then calls `open_original_encyclopedia_at`
   with that id, as for any listed object. Remove
   `open_original_encyclopedia_unlisted`, `original_extra_entry` and the
   `extra` part of `window_entries`.
4. Tests: a fresh index (no fleet opened) lists Fleet in All Databases
   between Flax and Fwillsving, and in no other category. Rework
   `an_unnamed_objects_topic_joins_the_window_list_by_its_name` to the
   listed id; its previous and next steps stay the same.
5. The catalog count rises by one (the P65 356-entry audit). Change that
   count with this trace as the cause.

## A0 capture needed

One capture would confirm it. Open the Encyclopedia from the cockpit
(F7), with no fleet's Encyclopedia opened first. Stay on All Databases
and scroll to F. The trace predicts "Fleet" between "Flax" and
"Fwillsving". A second capture of Ships Database (`0x71`) at F should
show no Fleet row.

## Port fix

2026-11-12. `load_encyclopedia_catalog` now reads `FLEETSD.DAT` and
pushes each record of family `0x08..0x0f` (`push_fleet_entries`, citing
`FUN_00422620`): only record 4, `0x0800_0004`, "Fleet". The catalog has
357 entries (all:357); the category counts are unchanged.
`FLEET_TOPIC_OBJECT_ID` is now `0x0800_0004` and binds through
`bind_encyclopedia_topics`; `fleet_topic_binding` is gone. A fleet's
Encyclopedia opens through `original_encyclopedia_object` (now
`Some(0x0800_0004)` for a fleet) and `open_original_encyclopedia_at`;
`open_original_encyclopedia_unlisted`, `original_extra_entry` and the
`extra` part of `window_entries` are removed. `FLEETSD.DAT` joins the
legacy browser data list (the runtime pack already ships every `*.DAT`).

Counts changed with this trace as the cause: the P65 audit
(`encyclopedia-source-audit`) 356 → 357 index entries, 346 → 347
complete topics and 172 → 173 distinct bound artwork per side, unbound
texts `[7176, 7427]` → `[7176]`, unbound artwork
`[7188, 7427, 11284, 11523]` → `[7188, 11284]` (Fleet binds `0x1d03` /
`0x2d03`, `EDATA.132` / `EDATA.133`, measured on the owned source). The
gate `encyclopedia-index-shell.mjs` expects `entries=357 ... all:357`.

Tests: `a_fresh_index_lists_fleet_under_all_databases_and_not_under_ships`
and `the_installed_index_lists_fleet_under_all_databases_when_present`
fail with the FLEETSD filter disabled and pass with it;
`the_fleet_topic_steps_through_all_databases_by_its_name` replaces the
unlisted-entry test. No WASM rebuild, gate or A0 capture yet; the A0
capture above is still open.

## Batch b198

The full gate batch b198 (2026-11-12 10:16-11:30) passed 29 of 30 gates, the workspace (3620 tests) and fixture tests (158), and 172 tactical battle checks, 0 failed. The one failure was fleet-window.mjs fleet-encyclopedia, which still expected the old fleet object 0x0800_0000; the expectation now names the class 0x0800_0004 (FUN_004f7d50) and both sides pass (run fleet-window-2026-11-12T10-36-04-649Z-764990).
