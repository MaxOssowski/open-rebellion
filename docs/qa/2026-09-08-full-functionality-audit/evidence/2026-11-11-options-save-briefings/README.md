# Options, save and briefing cells against A0 (2026-11-11)

This covers the partial cells PRE-01-C009, PRE-02-C002, PRE-03-C008..C010 and
PRE-05-C003..C005. Only PRE-05-C003 (the droid animation) gained new A0
evidence. The others need A0 captures that do not exist yet; they are listed
at the end.

## PRE-05-C003: the droid animation

### A0

The briefing frames from 2026-11-05 (`.artifacts/a0-vm/briefing-alliance/`
`b000..b139`, 86 frames, a screen every 1.5 s, then 3 s;
`.artifacts/a0-vm/briefing-empire/` `e000..e098`, 99 frames, every 2 s).
Their hashes are in `../2026-11-05-briefing-a0/a0-sha256.txt`.

The talking droid is the only thing that changes between these frames in its
corner. The boxes are:

- Alliance: C-3PO, x 536..606, y 344..452.
- Empire: the Imperial droid on its pad, x 28..110, y 352..480.

### The gate

`tools/interface-parity/briefing.mjs`, case `full`, now samples the droid's
box in the port on every loop pass, from the first voice to action `0xd`
(about 2,500 distinct crops per side). Each A0 frame's box is then compared
with every sampled crop, pixel for pixel. Frame `b000` / `e000` is left out:
A0 took it before the first voice. The nearest port crop of each miss is
saved as `droid-nearest-<A0 file>` in the run's directory.

- **Alliance: passes.** 81 of 85 A0 frames match a port frame exactly. The
  rest are within 600 pixels of one (476, 239, 435 and 543 pixels in run
  `briefing-2026-11-11T18-33-08-861Z`). These are poses between two of the
  port's samples (the port is sampled about every 100 ms, A0 every 1.5 s),
  and `b121`, taken after `0xd` when the sampling has stopped. The gate
  asserts at least 0.9 exact and every miss within 600 pixels.
- **Empire: differs.** No A0 frame matches. Each is 990..1,190 pixels from
  its nearest port frame. The droid stands in the same place and uses the
  same colours. But the shading differs across its whole body, and some
  poses differ slightly (an arm further out in A0 `e005`). The differing
  pixels swap one palette colour for a neighbouring one, so this looks like
  different frame content, not a palette change. Two guesses, neither
  traced: the port builds the Empire clips' cumulative frames from another
  base frame, or A0 plays other clips. The nearest port crops are in
  `empire-full/droid-nearest-e00N.png` of each run. The gate keeps the
  numbers in `result.json` (`droid.open`) and does not assert them.

Proposed status: PRE-05-C003 stays **partial** (the Alliance passes, the
Empire's droid frames differ).

### Runs (Linux, Chromium 155, headless, `web/open-rebellion.wasm` of 26774184)

- `briefing-2026-11-11T18-28-04-644Z`: first probe, Alliance `full` only.
  It failed: the droid comparison ran before the closing checks, so the idle
  clip's voice (`alsprite-dll` 1219) had started by then. The comparison now
  runs last.
- `briefing-2026-11-11T18-33-08-861Z`: both sides, `full`. The Alliance
  passed; the Empire failed on the droid check, now left unasserted.
- `briefing-2026-11-11T18-42-50-871Z`: stopped by a shell timeout before
  its end (no result); its Empire `droid-nearest-*` crops were kept.
- `briefing-2026-11-11T18-52-29-091Z`: **all four cases pass** (Alliance
  and Empire, `full` and `skip`). WASM sha256 `8dd67825…6570f`. Alliance:
  83 of 85 exact, misses `b104` (435 pixels) and `b121` (543). Empire: 0 of
  98 exact, 1,024..1,190 pixels each (recorded, not asserted).

## Cells with no new A0 evidence

- **PRE-03-C008 (corrupt save).** No A0 capture shows a corrupt save. Needed:
  in the VM, put a truncated copy of a save (for example the first 100 bytes
  of a real `.sav`) into a save slot. Capture the Load list showing that slot,
  then the screen after pressing Load on it (any message box, with its text
  and caption).
- **PRE-03-C009 and C010 (music and effects sliders).** The art and the
  transfer are done. What is open is the audible level. Needed: an audio
  recording of A0 (for example QEMU `-audiodev wav`) of one music track and
  one effect at slider positions 0, 50 and 100, to compare with the port's
  gain.
- **PRE-05-C004 (narration).** Needed: an audio recording of A0's whole
  briefing for each side, with a clock, to compare when each voice starts.
- **PRE-05-C005 (skip).** Needed: A0 frames every 0.5 s of a new game with
  the briefing DLL restored, with one click on the galaxy view while the
  first voice plays, until the Message Index opens; and audio, to hear the
  first voice cut and the short-form voice.
- **PRE-01-C009 (media failure).** The box is already compared with A0. The
  open part is the sound of `MessageBeep(0x10)`; it needs an A0 audio
  recording with a movie file missing.
- **PRE-02-C002 (14 control hotspots).** Needed: A0 hit probes like
  `main-menu-hit/` (a press on each control's keyed corner and on an opaque
  pixel) on the galaxy-size screens, Game Type, Load/Options, Credits and
  Multiplayer.
