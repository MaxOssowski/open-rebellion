# Force Growth took the class's portrait, not Luke's own

Date: 2026-11-08. Adds to EVT-01-C014 and to the 2026-11-08 Luke portrait
fix (b133).

## A0

`.artifacts/a0-vm/alliance-game2/luke-force-growth.png` (SHA-256
`4de801ee4a75a4638d474b66d995963c7b70d5e9e49433d805103bcc14491e2d`) comes
from an Alliance game in the Windows 10 VM, day 643. Luke had gone to
Dagobah on about day 421.

The message is "Luke Skywalker Force Growth": "My recent experience has
given me new insights and I have grown in my skills in the Force.  My
ranking is Jedi Student." (`FUN_0048ed80`, `0x7180` / `0x7181`).

The portrait matches STRATEGY 6224, the Jedi Knight, in every sampled
pixel. The young Luke, 6210, differs in 5208 of 5267 sampled pixels.

## The defect

b133 made the story builders take a character's own picture
(`character_portrait_of`, which asks `FUN_0056f550` for Luke). The story
classes' `StoryClass::picture` still took the class's portrait from
`(dat_id, major)`, so Force Growth showed the young Luke after Dagobah.

`story_message` now replaces the portrait with the subject's own. The
test `a_story_event_posts_its_classes_message_or_keeps_the_port_text`
fails without the change.

## Browser

The story fixture (Alliance) files Luke's Force Growth with Luke past
Dagobah, on a copy of the world. `message-display.mjs` (Linux) checks:

- the start of the body;
- the picture, title image and rail, each 0 pixels from A0.

The ranking word follows Luke's Force level, so the whole body is not
compared.
