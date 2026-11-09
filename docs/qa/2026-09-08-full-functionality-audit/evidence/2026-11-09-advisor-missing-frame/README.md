# A missing clip frame holds the picture; the clip plays on

Date: 2026-11-09. Settles CMD-07-C012 ("missing frame"), listed as "not
settled" in `2026-11-08-states-absent-from-the-original`.

## The trace

An op-4 clip definition is `[sound, wave, wave slot, count, first]`
(`ghidra/notes/advisor-briefing.md`). Its loader `FUN_00404440` builds the
frame list from it:

- the first frame is a bitmap (`FUN_005fbd20(module, first, 2)`);
- frames 1 to `count - 1` are type-302 deltas `first + 1 ..`, each made by
  `FUN_0041c590`, which calls `FUN_0041c680(module, id)`.

`FUN_0041c680` clears the frame (`FUN_0041c8f0`: width `+0x1c` and height
`+0x1e` 0, no row table), then calls `FindResourceA(module, id, 0x12e)`.
Only when the resource is found does it parse it (`FUN_0041c6c0`). A
missing frame therefore stays in the list as an empty, 0 by 0 frame.

Each tick `FUN_00404360` applies the next frame with `FUN_0041c7a0` and
shows the result (`FUN_0041ca10`). `FUN_0041c7a0` returns at once when
the frame's width or height is not the bitmap's, and its row loop runs
`+0x1e` times, so an empty frame changes nothing either way. The picture holds for that tick, and the clip goes on
to its next frame. It still ends when its frames and its voice are done.

## The port before

`advisor_clip.rs` ended the clip at once when a frame could not be read
(`return true`). That ran the clip's action early and cut its voice off.

## The change

A frame that cannot be read now changes nothing: the picture stays and the
clip plays on. The test
`a_missing_frame_holds_the_picture_and_the_clip_plays_to_its_end` removes
frame 2103 of a four-frame fixture and checks:

- 2102 is applied;
- the picture and its serial hold for 2103's tick;
- 2104 is applied on top of the held picture;
- the clip ends with its voice.

The test fails without the change.

## Scope

The shipped data never plays a clip with a missing frame. Every
definition the briefing plays has all its deltas. The two with missing
deltas, ALBRIEF `0x103c` and `0x1042`, are never queued. So no A0 capture
can show this state without editing a DLL. The cell stays partial, with
the trace and the unit test.

A missing first bitmap is not changed. In the original, `FUN_005fbd20`
then leaves an empty bitmap, and the voice still starts. The port still
skips such a clip and runs its action at once. What the aperture shows in
that case is not traced.

## Batch b147

23 of 23 interface gates on Linux; workspace tests 3492 passed; the
diff's 4 mutants (this file and `bmp_cache.rs`) caught.
