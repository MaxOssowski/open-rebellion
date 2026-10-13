# Message pictures, browser check (2026-10-13)

Scope: the pictures `rebellion-render/src/message_pictures.rs` attaches to
the port's report messages, drawn by the Message window's single-message
mode. Source: `ghidra/notes/message-pictures.md`.

## Run

- Fixture build (`--features interface-test-fixtures`), scenario 56
  (`EventMessages`), Alliance: `?fixture-code=313`. WASM sha256
  `8640ee8f30d88fdcc9540d52cb8ec7b1a269eaa71263de196704099d1f8e11af`.
- Runtime pack built with `scripts/build-runtime-pack.py --edata`: 2326
  bitmaps and 187 Encyclopedia assets (console log).
- Headless Chromium through the DevTools protocol: click the lit Mission
  category, Display, then the down arrow through the list.

## Seen

| Message | Picture drawn |
|---|---|
| Mission Report (Diplomacy, Leia leading) | STRATEGY `0x414` with Leia's portrait `0x1841` centred on it |
| Uprising Begins | `0x3f2` |
| Uprising Ends (Alliance system) | `0x3f3` |
| Blockade (Alliance system) | `0x404` |
| Fleet Arrives (Alliance reader) | `0x3fa` |
| Units Arrive (Alliance reader) | `0x3fc` |
| Shipyard Deployed | Encyclopedia artwork `0x1100`, `EDATA.003` |
| Regiment Deployed (Alliance Army Regiment) | Encyclopedia artwork `0x1441`, `EDATA.016` |
| Character Killed (Alliance) | `0x42e` |
| Research Complete | `0x3f9` |

Each fills the 400 by 200 picture area with the title above and the text
below. The two Encyclopedia pictures prove the fallback for ids STRATEGY
does not hold.

## Defect found and closed

The Assault picture (`0x2b98`, 400 by 310) ran past the window's lower
edge and under the text (`browser-alliance-assault-overflow.png`). The
Assault and bombardment pictures were withdrawn from the port's messages
and from the scenario; the capture predates that, so the build above still
listed them.

## Not covered

- Empire reader; a native run.
- The messages as the campaign posts them: a live game looped on a battle
  alert at Ithor before any report could be read.
- Pixel comparison with the original.

The screenshots show original artwork, so they are not committed. They and
the console log are in `target/acceptance/message-pictures/`; their
checksums are in `SHA256SUMS`.
