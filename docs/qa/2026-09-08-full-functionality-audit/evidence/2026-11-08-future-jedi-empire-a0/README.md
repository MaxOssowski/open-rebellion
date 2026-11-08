# Future Jedi Discovered for the Empire, against A0

Date: 2026-11-08. Closes EVT-01-C014.

## A0

The capture is `.artifacts/a0-vm/research-display/08-future-jedi.png`. It is
from a new Easy Empire game in the Windows 10 VM, day 107. Vader reports:
"I have detected that Ozzel has the ability to use the Force.  We should
consider allowing me to conduct training in the way of the Force."

- The builder is `FUN_0048f1e0`, sent by notification `0x362`.
- The title is `0x7178` and the body `0x7179`, because Vader trains at once.
- The picture is the Empire's background with the trainer's portrait.
- The window stands at (125, 52).

## Port

- The event-message fixture (`interface_test_fixture::force_discoveries`)
  names Vader and Ozzel for an Empire reader.
- The app files the message through
  `story_messages::force_user_discovered`.
- The fixture's unit test is
  `the_event_messages_scenario_has_vader_find_ozzels_force_for_the_empire`.

## Display was grey; A0 shows it lit

The first run differed from A0 in 613 rail pixels, all in the Display
button (0x67). A0 shows it lit, and the port showed it grey.

- Class `0x0065b520`'s vtable slot `+0x10` is the generic `0x5a6730`,
  read from REBEXE.EXE. It returns kind 3.
- For kind 3, `FUN_004697b0` lights Display whatever the target.
- `FUN_0048f1e0` sets no object at `+0x5c`. A press therefore reaches
  `FUN_004ece60` with an empty id, and `FUN_00467f10` goes nowhere.

The port now marks the message `with_unplaced_target()`: Display is lit
and its press does nothing. The unit test
`a_discovery_tells_the_trainers_side_what_it_found` checks this. It fails
without the change.

Other kind-3 classes may post without a system in the port. Their Display
was not checked here.

## Gate

`message-display.mjs` (Empire, Linux Chromium) opens the row, shows the
message and checks:

- the exact text;
- the picture (window 12,33, 400x200);
- the title image (15,16, 15x16);
- the rail (414..468, 11..296).

Each of these is 0 pixels from A0. The only pixels left out are the ones
under A0's pointer, at screen x 228..252 and y 228..262.

Leia Uses Force (`0x71d8`) has no A0 capture.
