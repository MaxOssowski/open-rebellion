# Character Captured and Escaped: which side reads which case (2026-11-07)

This covers EVT-01-C011 (capture) and corrects `../2026-11-06-escape-and-arrivals/`.

## A0 in an unedited game

A new Easy Empire game, day 328, in `.artifacts/a0-vm/empire-clean/`:
- `30-day328-all-messages` (sha256 `2306fa45c7ee0be32e19b8a38f274f0ab855fe97947842f403b493547cfd3fbb`) lists "Needa Captured";
- `31-needa-captured` (sha256 `2b294dd882c000b881e9398358802f05b0058caad2810cb0580064446b96a45f`) shows "Needa was captured by the Alliance at Umgul." over STRATEGY 1049 (`0x419`, a captive of side 2), 0 differing pixels.

The Empire reads its own officer's capture with `0x7158` / `0x7159` ("by the {3}"), case 3.

## The source

`FUN_00490340` sets `bVar8 = (FUN_004edc80() == 1)` and inverts it for reader 2: case 3 (or case 1 for an escape) when the tested side is the reader's. With the captive's side, Needa (side 2) read by side 2 is case 3, as A0 shows.

The port had read the side the other way, from `messages-day633/capture-palpatine`: "Emperor Palpatine was captured by the Alliance at Corfai." in an Alliance cockpit. That capture comes from the DS_flip_allian game, which builds its messages for the Empire behind an Alliance cockpit. So it is the Empire reading its own Emperor's capture: case 3 again.

## Now

| Message | Captive's or escapee's own side | The other side |
|---|---|---|
| Captured | `0x7158` / `0x7159` "{1} was captured by the {3} at {2}." | `0x715a` / `0x715b` "{1} was captured at {2}." |
| Escaped | `0x715c` / `0x715d` "{1} escaped from the {3} at {2}." | `0x715e` / `0x715f` "{1} has escaped from imprisonment at {2}." |

- No escape has been captured in A0. The escape follows the same side test (hyp).
- Pictures and speakers are unchanged.

**Tests.** These now assert the corrected cases:
- `a_capture_reads_the_captor_to_its_own_side_and_the_place_to_the_other`;
- `an_escape_reads_from_the_captors_side_and_the_escapees`;
- `an_escape_frees_the_captive_and_tells_both_sides`;
- `the_event_messages_scenario_files_the_emperors_capture`.

Each assertion changed only with this evidence.

## Browser (`message-display.mjs`, Linux)

- The Alliance now reads the fixture's Emperor capture as "Emperor Palpatine was captured at Corfai.", and the Empire as "... captured by the Alliance at Corfai.".
- The Alliance-side A0 checks of `capture-palpatine` stay: picture, title image, title ink and rail.
- The body's ink box moves to the Empire side, whose text is A0's: `[18, 237, 347, 249]` against A0's `[18, 237, 348, 249]`.

Screenshot: `.artifacts/interface-parity/message-display-2026-11-07T05-23-15-802Z/empire/capture.png` (sha256 `5095624223eb89ca7e8e9daade7a5707c38465beb1a4212f21fa0ca39a8d3f54`).
