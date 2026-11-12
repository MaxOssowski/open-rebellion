# Encyclopedia: can any path reach a missing entry?

Date: 2026-11-12. Cell: OBJ-01-C013 (missing entry). This is a static
trace only. No code, WASM, gate or VM run. It does not edit the ledger.

## What was asked

The cell is partial. Its open question: can another path reach an
object with no Encyclopedia entry? For example, an object menu's
Encyclopedia on a hidden mission (Move, Return, Autorouting, Adrift,
Dagobah, Palace, Vacation, Sabbatical, Pickup, Bounty). If so, what does
the original show?

Sources: `~/Games/rebexe-decompile/decompiled-typed/FUN_<addr>.c`,
`ghidra/notes/encyclopedia-window.md`, `status-window.md`,
`mission-dialog.md`, `manufacturing-build-selection.md`.

## The trace

### Every way in

`FUN_0045d400` (the window's constructor) has one caller,
`FUN_00429f30`. That function has two callers:

| Caller | Path | Ids passed |
| --- | --- | --- |
| `FUN_00422ce0` case `0x131` | cockpit button, F7 | both null |
| `FUN_00422ce0` case `0x76` | galaxy view command | both null |
| `FUN_0041d6b0` | the five below | object id, class id |

`FUN_0041d6b0` has five callers:

| Caller | Window | What it passes |
| --- | --- | --- |
| `FUN_00486fb0` command `0x100` | object menu's Encyclopedia | the selection's object id, null class |
| `FUN_0046c3c0` case `0x67` | Mission dialog's Encyclopedia | the chosen row's id `+0x54` |
| `FUN_00438800` case `0x67` | Build Selection's Encyclopedia | the chosen row's id `+0x54` |
| `FUN_004443a0` | Status window's Encyclopedia | null object, the window's id `+0x128` |
| `FUN_00467f10` (answer 4) | Message Index, kind-4 message | the message's id `piVar7+0x18` |

No other function calls `FUN_0045d400`, `FUN_00429f30` or
`FUN_0041d6b0` in the corpus.

### How the constructor picks the first view

`FUN_0045d400`:

1. With an object id, it looks the object up (`FUN_004f2d10` on the
   galaxy's table `+0x9c`). A missing object clears the id.
2. With only a class id, it resolves the class (`FUN_0051cab0`). A
   missing class clears the id.
3. With both null, `+0x148` (the selected list item) = 0.
4. Otherwise it searches the galaxy view's item list `+0x474` with
   `FUN_0060a860` for the key `(name resource +0x30 & 0xfff) + 0x1000`.
   The name comes from the class record, or the object's `+0x2c`
   record. `FUN_0060a860` walks the list and returns the item whose
   `+0xc` equals the key, or 0.
5. If the search fails, `FUN_0045fd90` maps a production manager
   (family `0xa0..0xaf`) to another id and searches once more. Any
   other id searches with the same key and fails again.
6. At `0x0045d69d`: if `+0x148` is 0, `+0x114` (the mode) = 1, the
   index.

`FUN_0045ddc0` (layout) then presses Topic `0x67` only if `+0x114 == 2`
and `+0x148 != 0`. Otherwise it presses Index `0x68`.

### What the list holds

`FUN_00422620` builds `+0x474` once. It takes the class records of the
listed families. For missions (`0x50..0x7f`) it adds a record only when
its `+0x5c` is 0. The ten hidden missions have `+0x5c` set, so their
keys are never in `+0x474`. Then it adds the viewer's systems.

### Inside the window

`+0x148` changes only to items of the window's own list:

- `FUN_0045da70` case `0x65` (row click), `0x83`/`0x84` (previous,
  next: the item's `+0x10`/`+0xc` links);
- `FUN_0045d8f0` (the Topic box, `FUN_006098f0`);
- `FUN_0045f100` (category change) sets it to 0.

`FUN_0045f100` fills every category from `+0x474` alone.
`FUN_0045f480(2)` returns at once if `+0x148` is 0. So the topic view
can show only an item of `+0x474`.

### Does a listed item lack its entry?

P66A (`2026-10-01-encyclopedia-topic-source-bindings.md`): the ten
records without text and art are exactly the ten hidden missions. All
346 listed objects per side have both.

`FUN_0045fa60` would cope with a missing text: when `FindResourceA`
(type 10) returns 0, it skips the text update. Nothing else changes. No
reachable item takes that branch with the shipped `ENCYTEXT.DLL`.

### Answer for the original

No path shows a missing entry. An object menu's Encyclopedia on a hidden
mission, or on any object whose key is not listed, opens the index. It
shows All Databases with nothing selected, as the cockpit button does.
The same holds for the Mission dialog, Build Selection, Status window and
Message Index: they all go through the same lookup in `FUN_0045d400`.

## The port

- The window's catalog is built from `EncyclopediaCatalog::listed_entries`
  (`crates/rebellion-app/src/main.rs` ~1131), which drops
  `hidden_missions` (record `+0x5c`). This matches `+0x474`.
- Object menu `0x100` (`main.rs` ~6815): a mission's kind gives its
  MISSNSD record (`MissionKind::record_id`, Move `0x4100_0001`). Other
  objects use `original_encyclopedia_object`.
- `open_original_encyclopedia_at` (`crates/rebellion-render/src/encyclopedia.rs`
  ~1198) shows the topic only when `resolve_object` finds the id in
  All Databases. Otherwise it opens the index, All Databases, nothing
  selected. A hidden mission's record is not in the list and its family
  byte is set, so `resolve_object` returns None. The port opens the
  index, as the original does.
- Message Index kind 4 (`main.rs` ~5466) uses the same function.
- Test: `encyclopedia.rs` ~3612 opens an unknown character and asserts
  the index with nothing selected. No test names a hidden mission
  directly.

Side notes, outside this cell:

- The Mission dialog, Build Selection and Status window set only
  `enc_state.open = true` (`main.rs` ~7397, ~7443, ~7742). They do not
  reset to the index or open the chosen kind. This is the known
  deviation in `mission-dialog.md` ("without the selected kind's entry").
  It can reopen the last topic, but that topic is always a listed one.
  So it cannot show a missing entry.
- The fleet topic: the port shows "Fleet" through
  `open_original_encyclopedia_unlisted` and lists it only while that
  window is open (A0 `menus2/20`, `25`). By this trace, the original
  shows a topic only for a key in `+0x474`. So key `0x1d03` would be in
  the list from `FUN_00422620`, and then it would be in every index. No
  capture shows the plain index around "Flax". This is not traced here.

## Verdict

**Pass candidate.** In the original, no caller can show a topic for an
object without an entry. Every caller goes through `FUN_0045d400`. It
searches `+0x474`, which leaves the hidden missions out. On a miss it
opens the index (`+0x114 = 1`, `FUN_0045ddc0` presses `0x68`). The port
does the same through `listed_entries` and `resolve_object`.

What is left is not code: no A0 capture shows this case. The capture
proposed in `2026-11-12-droid-encyclopedia-quadrants/README.md` (a
mission's Encyclopedia during a Move) would confirm it. The trace
predicts the index, All Databases, nothing selected.
