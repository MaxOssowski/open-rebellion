# Cutscenes play in the browser (PRE-01, END-01; F-008)

Date: 2026-10-30. On 2026-10-30 the user asked why no film plays in the
browser.

## The cause

In the browser build `VideoPlayer` was a stub. `open` succeeded, then
`is_finished` was true at once, so every movie was skipped: the intro, the
story movies 101..108 and the trench run's 201/202. The native player
reads pre-decoded PNG frames and a WAV (3.4 GB), which a browser cannot
download.

## The port

- **`web/index.html`.** A new page plugin, `open_rebellion_video`, gives
  the game three calls:
  - `play(stem, volume)` puts a `<video>` of `data/video/<stem>.webm` over
    the canvas: full window, `object-fit: contain`, black around it,
    `pointer-events: none`. That is how the native player draws its frames.
  - `state()` returns idle, playing, ended or failed.
  - `stop()` ends the movie.
  
  When the browser refuses sound before any gesture, the movie plays muted
  rather than not at all. A file that fails to load counts as ended.
- **`video_player.rs`.** The wasm `VideoPlayer` drives that plugin.
  - `open` passes the movie's stem.
  - `advance` and `is_finished` read the page's state.
  - `stop` ends the movie.
  - `current_frame` is `None`, since the page draws the movie.
- **The build and the package.**
  - `scripts/build-wasm.sh` stages `assets/references/ref-videos/*.webm`
    (15 files, 268 MB) into `web/data/video/`. They are decoded from an
    original installation and never committed.
  - `scripts/package-web.sh` ships them beside the runtime pack. They load
    on demand.
- **`main.rs`.** The port's own "SPACE / ESC to skip" label is gone; the
  original draws no such text. Space and Escape still skip.

## Browser (Arch Chromium 155, release WASM `fc8ee618…`, 000.webm `0af15384…`)

The new gate `tools/interface-parity/cutscenes.mjs` serves `web/` and passes
both cases (`cutscenes-result.json`).

| Case | What it checks |
|---|---|
| `natural-end` | The intro 000 (640 × 480, 17.3 s) plays in the page's video with the main menu waiting, runs to its end, and the main menu follows. |
| `escape-skip` | The same intro, and Escape ends it at once; the main menu follows. |

Screenshots:
- natural-end: `playing` `148c7cf3…`, `main-menu` `abf1e2d2…`;
- escape-skip: `playing` `963a248d…`, `main-menu` `abf1e2d2…`.

## Still open

- **The intro sequence.** The port plays only 000 (the LucasArts logo). The
  original's sequence (developer logo, card, crawl, title, cinematic) and
  its skip rule are being traced.
- **The ending.** The end screens and the ending movies are not ported.
- **A0.** The intro is not yet compared with the original running in the
  VM.

## Tests

- `cargo test --workspace`: all pass.
- The wasm player is browser code; the gate covers it.
