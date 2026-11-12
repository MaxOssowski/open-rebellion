# The Message Summary arrows: which one goes to the older message

Date: 2026-11-12. Trace only. No code, build or gate was run. Adds to
CMD-08-C004 ("summary").

## What was asked

CMD-08-C004 is partial. Its note says: "which arrow goes to the older
message is still inferred". The arrows are controls `0x96` and `0x9a` in
the Message Index's mode 2, the open message (manual p. 79, Fig. 3.19).
The task: settle the direction from the original's code, and check the
port against it.

Earlier evidence:
`2026-11-09-summary-headline-and-message-arrows/README.md` inferred the
direction from the art, and the 2026-11-08 VM handling notes agreed.

## The trace

All addresses are in the GOG `REBEXE.EXE`. Sources: the typed decompile
corpus (`~/Games/rebexe-decompile/decompiled-typed/FUN_<addr>.c`), its
disassembly, and vtables read from the executable's bytes.

### 1. The buttons and their art

`FUN_004665f0` builds both buttons with `FUN_00602150` (size 19 x 15,
y 15):

| Control | x | Normal | Pressed | Disabled (`FUN_00603150(2, ..)`) |
| --- | --- | --- | --- | --- |
| `0x96` | `0x186` (390) | `0x2aa7` (10919, down) | `0x2aa8` (10920) | `0x2aa9` (10921) |
| `0x9a` | `0x16f` (367) | `0x2ac4` (10948, up) | `0x2ac5` (10949) | `0x2ac6` (10950) |

### 2. What each press does

The window's command handler is `FUN_00467f10` (`Class_0065a1c0_vt48`).
`param_1[0x5c]` (`+0x170`) is the open message's list row.

- `case 0x96`: calls the row's vtable slot `+0xc`. If that returns a row,
  it becomes the open row (`FUN_00469de0`).
- `case 0x9a`: the same with slot `+0x10`.

Both first mark the message being left as read (`row+0x68` -> `+0x24 = 1`,
`row+0x38 = 10`).

### 3. The row's slots `+0xc` and `+0x10`

A row is built by `FUN_004ad6b0` with vtable `0x0065c090`. Its slots,
read from the executable: `+0xc` = `FUN_004ad730`, `+0x10` =
`FUN_004ad750`.

- `FUN_004ad730` (disassembly at `0x4ad730`): returns `row+8`, the row's
  next link. If that row is hidden (`+0x6c == 0`), it asks that row in
  turn.
- `FUN_004ad750`: the same with `row+4`, the previous link.

`FUN_00468f20` sets `+0x6c` to 1 only for rows in the selected category.
So both arrows skip the other categories, as the manual says.

In the row list's insert, `FUN_005f59f0`, `node+8` points towards the
tail and `node+4` towards the head. `FUN_0060a860` walks the list from
the head through slot `+0xc`. So `0x96` moves one row down the list and
`0x9a` one row up.

### 4. The list's order

The row list (`+0x16c`) is built in `FUN_004665f0` by `FUN_004ad620(4)`.
That is `FUN_0060a790` with mode 4 at `+0xc` (vtable `0x0065c080`).
Its compare, `FUN_0060a890`, tests only bits 1, 2 and 8. With mode 4 it
returns 0 (disassembly `0x60a890..0x60a904`). So `FUN_005f59f0` appends
every row at the tail. The rows keep the order in which
`FUN_00468ab0` adds them.

`FUN_00468ab0` (`0x468c10`..`0x468e91`) adds one row per message:

- it takes the message store from `FUN_0041d1b0`. That is
  `FUN_00436780`: `*(this+0xc) + 0x7c`;
- it starts at `FUN_005f5530`, a jump to `FUN_005f5080`: from the root,
  follow `+8` to the end;
- it moves on with `FUN_005f5c60`: go to `+4`, then follow `+8` to the
  end; or climb `+0xc` until it comes up from a `+8` child.

The store is a balanced tree (`FUN_005f5100`, `FUN_005f5190`,
`FUN_005f5200`). `FUN_005f5080` returns the right-most node.
`FUN_005f5c60` returns the in-order predecessor. So the rows run from the
largest key down.

### 5. The store's key

The object at `*(this+0xc)` holds the message manager at `+0x6c`
(`FUN_00436790`..`FUN_004369bc` all pass `lea 0x6c(%eax)`). Its
constructor, `FUN_004888f0`, builds the tree at manager `+0x10`
(that is `+0x7c`) with `FUN_00436a40`. That sets vtable `0x00658da0`.
Slot `+8` there is `FUN_005f5560`: `a+0x18 < b+0x18`.

`FUN_005f4f10` (insert) puts a node to the left (`+4`) when slot `+8`
returns true. So smaller keys go left.

Messages are filed by `FUN_0048a060`. Kinds 3, 4, 5 and 8 go to manager
`+0x10` (`lea 0x10(%edi),%ecx` at `0x48a0e7`, then `FUN_005f5440`).
`FUN_005f5440` gives a message with key 0 the next number from the
store's counter (`+8`), then adds one (`0x5f544d`..`0x5f5466`). A message
that already has a key keeps it, and the counter moves above it. So a
newer message has a larger key.

### 6. Result

- The list's top row is the newest message. Each row down is older.
- `0x96` (x 390, down arrow 10919) opens the next row down: the **older**
  message.
- `0x9a` (x 367, up arrow 10948) opens the next row up: the **newer**
  message.
- Both step only through the selected category's rows.

This agrees with the art and with the 2026-11-08 VM handling notes
((461, 70) older, (436, 70) newer). The right-hand point falls on
`0x96`, the left-hand point on `0x9a`.

### 7. The disabled state

`FUN_0046a200` runs in mode 2. It calls the button's slot `+0xc` when
the row has a neighbour on that side, else slot `+0x10`. On the button
vtable `0x0066df58`:

- `+0xc` = `FUN_00602880`: clears state bit 2 (`FUN_006030f0`) and
  `EnableWindow(.., 1)` (`FUN_005ffe60`);
- `+0x10` = `FUN_006028a0`: sets state bit 2 (`FUN_006030c0`) and
  `EnableWindow(.., 0)` (`FUN_005ffe80`).

State 2 is the art set by `FUN_00603150(2, ..)`. So on the oldest
message `0x96` shows 10921 and does nothing. On the newest message
`0x9a` shows 10950 and does nothing.

## The port

`crates/rebellion-render/src/message_index.rs`:

- `MessageLog::push` (`message_log.rs`) appends with a rising id.
  `filtered_messages` reads the log with `.rev()`, so its rows run
  newest first. It keeps only the selected category, like
  `FUN_00468f20`.
- The click handler maps `0x96` to `view_adjacent(.., forward = true)`.
  That opens `rows[current + 1]`, the next row down, the older message.
  `0x9a` opens `rows[current - 1]`, the newer message.
- At either end `view_adjacent` returns `None`, so the press does
  nothing, as in the original.

So the direction matches the trace.

Two smaller gaps:

1. The doc comment on `view_adjacent` and the test
   `the_scroll_buttons_step_through_the_categorys_messages` still say
   "inferred from the art". They should cite `FUN_00467f10`,
   `FUN_004ad730`/`FUN_004ad750` and `FUN_00468ab0`.
2. The port has no disabled art for the arrows. The button list
   (`(0x96, 390, 10_919), (0x9a, 367, 10_948)`) has only normal and
   pressed resources. 10921 and 10950 are never drawn on these buttons.
   On the oldest or newest message the original shows the disabled art.

Fix for gap 2 (not done here): in mode 2, when the open message has no
older row, draw `0x96` with 10921; when it has no newer row, draw `0x9a`
with 10950. Do not show the pressed art for a disabled button. The art
must be keyed like 10919/10948 (see `bmp_cache.rs`); the keying of 10921
and 10950 needs an A0 check first. A test: open the newest message and
expect `0x9a`'s resource to be 10950.

## Verdict

Pass candidate for the direction. The code settles it: `0x96` goes to
the older message and `0x9a` to the newer one. The port does the same.

Still open in this cell: the disabled arrow art (gap 2). No A0 capture
is needed for the direction. For gap 2, one A0 capture of the newest
message open (to see 10950 on `0x9a`) would confirm the art and its
keying.
