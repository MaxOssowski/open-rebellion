# Informants Provide Information against an unedited Alliance game (2026-11-07)

This covers EVT-01-C001 (informant).

## A0

The unedited "test" save (Alliance, days 1456 and 1457):
- `.artifacts/a0-vm/informants-clean/test-day1456-index.png` (sha256 `c19b69e021515e0a06a829546522c34b4083a7f5fafb2d68110f3faf9e04cd88`) lists the message first;
- `test-day1457-selonia.png` (sha256 `56e56ef678919930ceed44536a0f1f6dc8c32965f0b534f9f3bda802c6fb70c8`) shows "Informants Provide Information" over "Informants have provided us with information about Selonia.".

The picture is STRATEGY 1000 (`0x3e8`), exact at (74, 82). The title's image is the Popular Support rail's. Display is lit.

## The port

`FUN_00491060` (notification `0x153`) was already ported with these texts, this picture, advisor event `0x1c` and the system. Its posting moves into `informants_message` so the fixture can post it too. The unit test `informants_tell_their_side_of_the_system_on_the_popular_support_rail` checks the rail, the reader, the picture, the event and the system.

## Browser (`message-display.mjs`, Linux, Alliance)

The EventMessages fixture files Selonia's informants first, so they list last on the Popular Support rail. The gate opens the message and finds A0's text with background `0x3e8`. Compared with A0, 0 pixels differ in each of these:
- the picture;
- the title's item image;
- the rail, Display lit.

Screenshot: `.artifacts/interface-parity/message-display-2026-11-07T02-38-59-576Z/alliance/informants.png` (sha256 `9dce2e5ed7608ea7049a6967978913c90bb82b26860b18d80de5a7ba5b234a24`).
