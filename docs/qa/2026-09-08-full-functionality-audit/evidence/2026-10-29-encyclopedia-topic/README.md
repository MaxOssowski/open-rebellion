# The Encyclopedia's topic view against the original (P35)

Date: 2026-10-29. Tracker item "Encyklopedia", the comparison of the
topic view left after the index (`../2026-10-29-encyclopedia-placement/`).

## Captures

- **A0.** Original game, Windows 10 VM, Empire, day 166: Abduction's topic,
  opened from the index with the rail's `0x67`.
  `topic-abduction-empire-day166.png` stays outside Git; its hash is in
  `a0-captures.sha256`.
- **Port.** `tools/interface-parity/encyclopedia-placement.mjs` now also:
  1. scrolls the list back to the top;
  2. selects Abduction;
  3. opens its topic from the rail, waiting for the log line
     `command=0x67 topic_mode=true`.

  Run `encyclopedia-placement-2026-10-29T11-36-20-993Z`. The WASM hash is
  `db32aade…`. The local runtime pack was built with the GOG EData
  (`REBELLION_EDATA_DIR`). That pack is not committed.

## Result

- **The picture.** EDATA at (12, 31), 400 by 200: 0 of 80,000 pixels
  differ (`picture-diff.txt`).
- **The title.** A0 draws it bold, in the title label `+0x120`
  (`FUN_0045fa60`'s `FUN_00601aa0`, font 5, centred in (36, 14, 350)). The
  port drew a regular stand-in and now draws font 5.
- **The body.** `FUN_0041ecf0` at (17, 231), 395 by 80. Its font is not
  read, so it was measured from A0:
  - lines 16 apart, from y 234;
  - the first line 374 pixels wide, which is Arial at a 13-pixel em
    (Liberation Sans measures 372).

  The port now lays out the body at a 13-pixel em with 16-pixel lines
  (hyp: from that measurement).
  - Lines 1 and 2 break as in A0.
  - Line 3 breaks one word earlier: Liberation Sans runs 2 pixels wider
    per line than GDI's hinted Arial.
  - The text sits 1 pixel higher.
  - These are font metric differences, recorded as such.

## Tests

- `cargo test --workspace`: all pass.
- `the_topic_lays_out_its_title_and_body_at_the_original_points` now
  checks:
  - the title in font 5, where it had the port's 15;
  - the body's 16-pixel line height and its colour.

  The body's em stays 13 (26 at scale 2).
- `cargo mutants --in-place --in-diff` on the topic change: 12 mutants, 4
  missed (the body format's colour and line height). The test was
  extended for them.
- Also fixed: the dead constant `ORIGINAL_INDEX_VISIBLE_ROWS`, left by the
  scroll bar stage. The list's 160 pixels now derive from it.
