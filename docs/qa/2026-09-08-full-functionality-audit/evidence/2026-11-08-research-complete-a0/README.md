# Research Complete against A0, and its Display from research missions

Date: 2026-11-08. Adds to CMD-11-C012 and CMD-08-C014.

## A0

A new Alliance game in the Windows 10 VM, day 204. The captures are in
`.artifacts/a0-vm/research-complete/`:

| File | SHA-256 | What it shows |
|---|---|---|
| `00-index-nebulon-chosen.png` | `bba419a95aae7178ce79d3f5cd00eeeceb78c102f636d75b7510d800531b8df4` | All Messages with "Nebulon-B Frigate Research Complete" chosen; the rail's Display in the Encyclopedia art |
| `01-nebulon-b-research-complete.png` | `1d22b11b852b6788c6b74417916b0ee962942dfd16ac205871122d69f330b502` | "R&D Reports that the Nebulon-B Frigate is now available for manufacture." |
| `02-display-opens-encyclopedia.png` | `239f8d2da52b00ba23589d314a6338f1eb81ab32aa6f80a14a8b7ecec616ac08` | Display closes the index and opens the Nebulon-B Frigate's Encyclopedia entry |

These are the first A0 captures of Research Complete, so kind 4 is now
confirmed in the original. Class `0x0065bb40`'s vtable `+0x10` is
`0x5a6700`, which returns 4. `FUN_00467f10` then calls `FUN_0041d6b0`.

## A defect found

The port posted Research Complete from two places:

- the economy's research tally;
- research missions.

Only the first set the message's Encyclopedia object. A Research Complete
from a research mission had no `+0x60` object, so Display treated it as a
kind-3 message rather than opening the Encyclopedia.

Both places now call the app's `research_complete_message`. It takes the
texts `0x7048` / `0x7049`, the picture, the advisor code 4, the
Encyclopedia object and the Manufacturing rail. The test
`a_research_complete_opens_its_class_in_the_encyclopedia_wherever_it_comes_from`
fails without the object.

## Browser

The event-message fixture (Alliance) files the ship-tree level that opens
the Nebulon-B Frigate through `research_complete_message`
(`interface_test_fixture::researched_class`). It lists last on the
Manufacturing rail.

`message-display.mjs` (Linux, Alliance) checks:

- the row chosen in the list: the rail, including Display in the
  Encyclopedia art, is 0 pixels from A0 `00`;
- the message shown: the exact text, and the picture, title image and rail
  are each 0 pixels from A0 `01`.

The Display press on the fixture's Research Complete opens the class's
topic and closes the index. That check runs on a fresh fixture, as before,
and A0 `02` shows the same.

Still open for CMD-11-C012: the `0x8031` "Encylopedia" tooltip has no A0
capture.
