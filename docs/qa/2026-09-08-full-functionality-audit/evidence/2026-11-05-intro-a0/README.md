# The intro and credits movies against A0 (2026-11-05)

This covers PRE-01 C001..C008, the movie flow of `FUN_004138a0` states 1 and 2, and PRE-04 C001..C004, Credits (`FUN_00406000`).

## A0

The Windows 10 VM, a fresh REBEXE launch. The frames are in `.artifacts/a0-vm/intro/`, with hashes in `a0-sha256.txt`.
- **`natural/`.** A screen every 0.37 s for 45 s, with no input.
  - MDATA.000: a light burst, the LucasArts logo (`f036`), then Coolhand (`f048`).
  - MDATA.001: the "A long time ago in a galaxy far, far away...." card (`f058`), the STAR WARS title shrinking away (`f078`), and the REBELLION crawl (`f114`).
  - About `f012`..`f030`, Windows' Xbox Game Bar hint covers the logo. Those frames are not used.
- **`menu/`.** The main menu, the shuttle, after MDATA.001 ended on its own with no input.
- **`skip/`.** Escape at 11 s, during the LucasArts logo, ends MDATA.000 and skips Coolhand. MDATA.001's card follows (`s026`..`s030`). A second Escape at 25 s, during the title, ends MDATA.001, and the main menu shows (`s060`..`s064`).

- **`credits/`.** The main menu's Credits button, STRATEGY 11271 at (459, 242), plays MDATA.005, the credit lines over footage. A screen is taken every 0.4 s, and a click at 16 s (`c038`) shows the menu again.
- **`credits/end/`.** Credits run to their end with no input: "Special Thanks to George Lucas", then the main menu.

## The port

The page plays `data/video/000.webm` (640 by 480) and `001.webm` (640 by 324) in a `<video>`. The 001 movie is centred, so its picture starts at row 78, as A0's does. The card's ink is in rows 202..276 in both.

## Tests

`cutscenes.mjs` (Linux Chromium, every case passes):
- **`a0-frames`** (new). For each of the five A0 frames, the case pauses the movie and steps it one frame (1/15 s) at a time across ±3 s of a time near it. It keeps the best screenshot. The share of pixels with every channel within 32 of A0's:

  | Frame | Port time | Share |
  |---|---|---|
  | LucasArts (`f036`) | 000 at 8.17 s | 0.9964 |
  | Coolhand (`f048`) | 000 at 14.53 s | 1.0 |
  | card (`f058`) | 001 at 0.60 s | 1.0 |
  | title (`f078`) | 001 at 8.10 s | 0.99997 |
  | crawl (`f114`) | 001 at 21.5 s | 1.0 |

  The case requires 0.98 for each.
- **`credits-and-new-game`.** Credits (command `0x73`) play 005. Four A0 `credits/` frames are found the same way, each with a share of 0.9994 or more (`c006` at 2.2 s, `c012` at 3.2 s, `c024` at 9.47 s, `c033` at 12.53 s). The movie's own end, seeked a second before it, shows the menu again, as in A0 `credits/end`. A second Credits ended by a click shows the menu too, as in A0 `c038`.
- **`natural-end`.** It now lets MDATA.001 end on its own: it seeks a second before the end. Before, it pressed Escape. The main menu follows, as in A0 `menu/`.
- **`skips`.** Escape on the logos starts 001, and a key on 001 shows the menu, as in A0 `skip/`.
- **The server.** The gate's server now answers byte ranges, so a paused `<video>` can seek.

## Open

- **PRE-01-C006.** The rest of MDATA.001, after the crawl's opening, is not compared frame by frame.
- **PRE-01-C009.** A missing movie file is not traced in the original.
