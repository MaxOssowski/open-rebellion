# A corrupt save's row and the droid's spare clip (2026-11-12)

Cells: PRE-03-C008 (corrupt save) and CMD-07-C012 (missing droid frame).

The decompile is the typed corpus
`~/Games/rebexe-decompile/decompiled-typed/FUN_<addr>.c` and
`ghidra/notes/`. The original files are the GOG install:

| File | SHA-256 |
| --- | --- |
| `REBEXE.EXE` | `b3fe3997cab9a6e96403d638875dcba25484e4d8601751afec748471ac0ed6ab` |
| `ALSPRITE.DLL` | `43a5083d28f06cf40ffbfcc5a7523da422ee11ab0605503e822400e897b847aa` |

## PRE-03-C008: corrupt save

### The trace (checked again)

- `FUN_00407180` builds the six rows. For each row it calls
  `FUN_00401860(header, slot)` and `FUN_00401910(slot)`.
- `FUN_00401860` starts from the default header (`FUN_00411730`): an
  empty name and six zero DWORDs at +0x10..+0x24. It then copies in
  what `FUN_00415cd0` read.
- `FUN_00415cd0` opens the slot's file (`FUN_005f4a60`). It reads the
  header (`FUN_00411900`: the name, then six DWORDs) only when
  `FUN_005f49b0` returns 0, that is when the open worked. Otherwise the
  defaults stay. A short read also leaves zeros.
- `FUN_00401910` -> `FUN_00415df0` returns the same header's +0x18.
- The Load button (`0x2741` / `0x2742`, x `0x11f`) is disabled (vtable
  +0x10) unless +0x18 is 1 (single player) and `FUN_00401910` is not 0.
- The crest needs +0x18 = 2 or a side of 1 or 2 at +0x20. A zero header
  has neither, so there is no crest.
- `FUN_00406b80`, commands 0x6f..0x74 (Save): the overwrite question
  `0x478` is asked only when `FUN_00401910` is not 0. With a zero header
  the save runs at once (`FUN_00408000`).

So an empty or unreadable save file shows as an empty row: no name, no
crest, Load disabled. A Save on it asks no overwrite question.

### The port before

`read_save_slots` (`crates/rebellion-app/src/main.rs`) listed such a slot
as "Unreadable save" / "Load to inspect error". Its Load was lit, and a
Save on it asked the overwrite question.

### The change

- New `rebellion_data::save::game_options_saves(dir, rows)`. It lists
  only the saves that read, in slot order, in the first `rows` rows. Its
  doc cites the FUN ids above.
- `read_save_slots` now calls it. An unreadable slot has no entry, so
  the Game Options window draws its row as an empty one. Its Load is
  disabled and a Save writes at once. The window's code already did this
  for an empty slot.
- The save format is unchanged. The unreadable file is not touched until
  the player saves over it.
- Tests:
  - `an_unreadable_or_empty_save_gives_an_empty_game_options_row` and
    `game_options_lists_only_its_rows_in_slot_order` (rebellion-data).
  - `unreadable_native_saves_show_an_empty_row_and_stay_unchanged`
    (rebellion-app) replaces `unreadable_native_saves_remain_occupied_and_unchanged`.
    Its old expectation ("Unreadable save") contradicts the trace above.
    With the old `read_save_slots` the new test fails (panic at the
    `slots.is_empty()` assertion).

### The browser smoke

`tools/interface-parity/game-options-smoke.mjs` now seeds:
- slot 2 with a zero-length save (both keys empty strings);
- slot 3 with a good header (slot 1's metadata) over a corrupt body.

It then checks, on both sides:
- row 2 is pixel-identical to the never-used row 5 (no name, no crest,
  grey Load);
- a press on row 2's Load changes nothing and loads nothing;
- one press on row 2's Save writes the slot (no question);
- row 3's Load fails and keeps the corrupt bytes.

A plain pixel match against row 5 does not work: the empty rows differ
from each other by a few hundred pixels. So the smoke counts marks
instead. A crest or a lit Load has saturated pixels (max - min channel
over 60); a name has white ones (every channel over 200). Row 2 must
have none, the same counts as row 5, and row 1 must have a name and a
lit Load.

Run `options-smoke-20261112T052914` (Linux, Chromium headless, 1280 by
960, production build from this tree) passes on both sides. The first
run, `options-smoke-20261112T052747`, failed on the pixel match above
and led to the mark count. In `empire-corrupt-load-error.png` row 2,
saved with an empty name, shows a lit Load and no crest: the crest
needs a name (`FUN_00407180`).

Seen on the screenshots: `alliance-empty-row.png` and
`empire-empty-row.png` show row 2 as blank as rows 4 to 6.

| File (run `options-smoke-20261112T052914`) | SHA-256 |
| --- | --- |
| `alliance-empty-row.png` | `745f56598d70b2075108efef1f683dceb4dd014f29a7342fc788ab0f1761e6bd` |
| `alliance-empty-row-saved.png` | `eae992d077db511b8eef548086f4fd5a89749f1db79974f3c244b0c0e90e2c9c` |
| `alliance-corrupt-load-error.png` | `ee6bf352ffb69d9486b00f9543c7f81544207e1c5a682a631589a1f597c1e58d` |
| `empire-empty-row.png` | `360c849687e3e86105a7263fe0051941dcdec85cbc81be9329f2a625f7cbe63b` |
| `empire-empty-row-saved.png` | `93e22264149cf3ebd943cc4e1a7a12fda2d9e3530e8c509dc1f77a032e0da896` |
| `empire-corrupt-load-error.png` | `fb074e9665a401eb7ce425d453f91632d571aca5034172c94ce4935a44e2c69a` |

### Mutants

- `game_options_saves` (rebellion-data): 6 mutants, 5 caught, 1
  unviable.
- `read_save_slots` (rebellion-app): the survivors are on the
  `timestamp` text and on `vec![]`. The window never reads `timestamp`
  or `game_tick` (no use in `game_options.rs`), so those are not
  observable. `vec![]` survives because no app unit test writes a
  readable save (that needs a full `SaveState`). The listing itself is
  `game_options_saves`, tested in rebellion-data, and the smoke shows a
  readable row 1 with its name and lit Load on both sides. The other
  survivors the regex picked up (`window_conf`,
  `draw_fullscreen_texture`) are outside this change.

### A save with a good header and a corrupt body

- `FUN_004013a0` -> `FUN_004158e0`. It does not load the file itself. It
  builds a 0x38-byte request (`FUN_004210c0`, class `0x00658ba0`, with
  the slot and the file name), serializes it into a buffer
  (`FUN_0051c300` / `FUN_0051c410`, via the class's vtable +0xc / +8)
  and posts it (`FUN_005970d0(buffer, size, 4)`, `FUN_00597170`).
- The load, and any error message, happen in the handler of that
  request. I did not find the handler. So the original's corrupt-body
  behaviour is still unknown. The port keeps its current behaviour: the
  row shows the name, Load is lit, and the load fails with an error and
  keeps the bytes.

### Proposed status

PRE-03-C008: the header-level case (empty or unreadable file) now
follows the decompile. It is not yet A0-backed. A0 needed (VM): put a
zero-length file in one slot and a truncated save (header intact, body
cut) in another. Capture Game Options, then press each row's Load and
Save.

## CMD-07-C012: the droid's clip 4155 / 10155

Round 4 found that every ALSPRITE / EMSPRITE clip has its first bitmap
and all its delta frames, except ALSPRITE 4155 / 10155.

- ALSPRITE RCDATA 10155 (`0x27ab`) is `[4, 0x103b]`. RCDATA 4155
  (`0x103b`) is `[0, 1155, 9, 137, 2101]`. It is ALBRIEF's
  `[0, 1155, 0xd, 137, 2101]` with slot 9 in place of 0xd. ALSPRITE has
  no BMP 2101 and no type-302 frames for it.

What can play a record from ALSPRITE (DLL slot 9):

1. **The advisor messages.** `FUN_004c0a60` / `FUN_004c2b20` play each
   entry through `FUN_0041d660` -> `FUN_0042b1d0(record, a, b,
   flags >> 1 & 1)`. `FUN_0042b1d0` reads the record from slot 9, or from
   slot 0xd when that bit is set, and builds the clip id from the same
   slot. I parsed both scripts in full (C3POACT.SPT 3510 bytes,
   IMP22ACT.SPT 3422 bytes). Record `0x27ab` appears once: C3POACT
   message 0x29 (the briefing), entry 1, flags 2. So it is read from slot
   0xd, ALBRIEF, never from ALSPRITE. No entry names `0x103b`.
2. **The idle-clip class** (`0x006587e8`, `FUN_00411b80`) queues its
   record's word 2 and `0x2d37` on the `0xbeef` player (`FUN_0041d690` ->
   `FUN_0042b290`, slot 9). No RCDATA record in ALSPRITE, EMSPRITE,
   ALBRIEF or EMBRIEF holds the word `0x27ab`. The only record holding
   `0x103b` is each DLL's own 10155 / 11155 `[4, 0x103b]`.
3. **Constants in the code.** The corpus has `0x27ab` once:
   `FUN_00426ee0` stores it at +0x2b8 in a table of galaxy-map marks
   `0x27a2..0x27ad` (read by `FUN_00426e70`, three sides of four marks).
   That is a bitmap id in another module, not a slot-9 record. There is
   no `0x103b`, `0x9103b`, `0x927ab`, `4155` or `10155` anywhere else.
   The idle clips are the constants `0x90192` / `0x90193`
   (`FUN_0042adb0`).
4. **A missing ALBRIEF.** With slot 0xd not loaded, `FUN_005fefd0(0xd)`
   gives a null module, so `FindResourceA` looks in REBEXE.EXE. REBEXE's
   RCDATA ids run `0x610..0x75e` (61 ids); it has no `0x27ab`. Nothing
   plays, and the lookup never falls back to ALSPRITE.

So nothing in the original plays ALSPRITE 4155 / 10155. It is a leftover
copy of the briefing's first clip. The missing-frame state cannot occur
in the original.

### Proposed status

CMD-07-C012: exclude as absent from the original (trace above). No A0
capture is needed.

## Batch b197

The full gate batch b197 (2026-11-12 08:23-09:45) passed: 30 gates, the workspace (3617 tests) and fixture tests (158), and 172 tactical battle checks in 43 scenarios, 0 failed. The ledger notes above were applied after it.
