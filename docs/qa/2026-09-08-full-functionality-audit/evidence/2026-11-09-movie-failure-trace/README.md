# What the original does when a movie cannot be opened

Date: 2026-11-09. Covers PRE-01-C009 (intro media failure) and END-01-C011
(ending media failure). This is a trace only. No code changed.

## The trace

Every movie goes through the video manager's play call `FUN_005f6920`.

1. It first asks for the movie's size with `FUN_005f6be0`. For a Smacker
   movie (kind 1) that is `FUN_005f71f0`: `SmackOpen(path, 0xfe000, -1)`.
   When the file opens, it reads the width and height and closes it. It
   returns whether the open succeeded.
2. When that fails, `FUN_005f6920` calls `FUN_0060a9e0(path, "PathName
   unknown to VideoManager")`. That is `MessageBeep(0x10)` and then
   `MessageBoxA(NULL, path, caption, 0x1010)`, a system-modal error box
   that shows the file's path and an OK button.
3. It then posts message `0x403` to the video manager's window. That is
   the same message a movie sends when it ends. No player is made.

So a missing or unreadable movie costs the player one system error box.
After it, the game gets the same end message a finished movie sends
(hyp: the intro and ending flows then go on as after a played movie; their
handlers of `0x403` were not read for this note).

The caption string is at file offset `0x2aa044` in REBEXE.EXE.

## The port

- Browser: `web/index.html` plays `data/video/<stem>.webm` in a `<video>`
  element. Its `error` event sets the state to FAILED, and
  `video_player.rs` treats FAILED as finished. The game goes on, as after
  `0x403`. No error box is shown.
- Native: `open_cutscene` (`rebellion-app`) skips the movie. It also posts
  an Event message, "cutscene skipped ...", to the Message Index. The
  original has no such message.

## Open

- The error box is not reproduced. A browser `alert()` would be the
  closest match. The port does not use one, because development builds
  that ship no movies would then show a box for every movie.
- The native port's extra Message Index entry has no counterpart in the
  original.
