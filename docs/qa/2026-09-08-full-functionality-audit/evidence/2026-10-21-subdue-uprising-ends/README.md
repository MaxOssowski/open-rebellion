# A subdued revolt posts Uprising Ends for both sides (0x14b)

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## The original

- A Subdue Uprising success ends the revolt through `FUN_0050c910`, which
  clears the system's uprising bit (`+0x88` bit 2).
- The bit's change posts `0x14b`, and Uprising Begins / Ends
  (`FUN_00499460`) speaks for both sides of the held system:
  - TEXTSTRA `0x7031` "Uprising Ends on {1:1}";
  - `0x7032` "The uprising on {2:1} has ended.".
- `38ffc9a3` ported these texts for the revolts `UprisingSystem` ends.

## What the port did before

The app's Subdue path posted its own line, "Uprising subdued at X", to
the holder only.

## Port

- The Subdue path posts `uprising_texts(name, holder, false)` for both
  sides.
- The port's line stays only when TEXTSTRA is missing.
- The `[interface] uprising_subdued` console line is unchanged. The
  2026-10-20 Subdue browser pass reads that line.

## Gates (`f39`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `2605f5506d2c677f0b3cc798ab22ea1a02945908b45b7fba7b4cdd9fcb2bf51a`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- The text function's test is in `38ffc9a3`. This change is in the app's
  mission loop only; no new Rust test, and the message was not checked
  in the browser.

Hashes of the committed files are in `SHA256SUMS`.
