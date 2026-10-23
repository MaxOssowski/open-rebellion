# The original Encyclopedia in production (P35)

Date: 2026-10-23.

## Change

- `rebellion_data::encyclopedia_topics::installed_encyclopedia_topics`
  binds each side's topics from the extracted source: native
  `data/base/encyclopedia/source.json` with its manifest; in the browser
  the runtime pack's `encyclopedia-source.json` /
  `encyclopedia-source.manifest.json`.
- `scripts/build-wasm.sh` stages the two files; `build-runtime-pack.py`
  packs them byte for byte (`RAW_TABLES`), since the manifest's checksum
  covers the catalog's exact bytes. The first try packed them as text
  tables, canonicalized, and the browser refused them ("checksum
  mismatch").
- The app draws the original index or topic (`FUN_00429f30`,
  `FUN_0045d400`) whenever the index catalog and the side's topics loaded;
  the rail's close `0xfb` closes it. Otherwise the replacement stays.
  A missing source logs at info level, not as an error.
- hyp: the window is centred in the 640 x 480 command centre;
  `FUN_0045d400` is built at `(0, 0)` of its parent, whose placement is
  not read. Contextual entry at an object is not ported. The
  replacement's "focus system" action has no original counterpart.

## Gates

- Workspace 2704 passed, 0 failed.
  `the_installed_source_binds_the_topics_when_present` binds 356 objects,
  346 complete topics per side, from the owned data.
- Runtime pack builder: 9 Python tests, including
  `test_encyclopedia_source_is_packed_byte_for_byte`.
- Mutants on the touched data code: 10 tested, 9 caught, 1 unviable.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152. The browser console shows the topics
  loading (`browser-console-encyclopedia.txt`: 356 entries, 346 complete
  per side).
- Not verified: a screenshot of the routed window. The fixture page did
  not react to F7 or a click on the cockpit's Encyclopedia button within
  the time left; the drawing is covered by the egui tests of the topic
  stage.
- Replay equivalence passed.
- Release WASM `2ffeb97f28b4d71816ceddceee7b1d2358781348c5512697010d81eef96b0171`.
