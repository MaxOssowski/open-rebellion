# Intro, end-of-game, credits movies in REBEXE.EXE (static RE, 2026-10-30)

Sources: ~/Games/rebexe-decompile/decompiled-typed, objdump of
~/Games/Heroic/Star Wars - Rebellion/REBEXE.EXE, MDATA headers, contact
sheets of assets/references/ref-videos/*.webm.

## Media files

MDATA\MDATA.NNN (no extension in the real name; "MDATA.000" etc.).
- 000..202: Smacker 2 ("SMK2" magic). 000 is 640x480, 259 frames (17.3 s);
  all others are 640x324 (letterbox, centred in the 640x480 client:
  FUN_005f6920 centres via GetClientRect).
- 300..315: RIFF WAVE music (FUN_00417520 plays PTR table at 0x6a8478,
  index < 16).
- MDATA.002 is referenced (id 0x67) but not shipped; no caller requests it.

## Movie id -> file (FUN_00421ba0)

0x65 000, 0x66 001, 0x67 002, 0x68 003, 0x69 004, 0x6a 106, 0x6b 105,
0x6c 107, 0x6d 108, 0x6e 005, 0x6f 101, 0x70 202, 0x71 201, 0x72 104,
0x73 102, 0x74 103.

## Player

- FUN_004146e0(app, id, caption, callback) / wrapper FUN_004019a0: destroys
  any previous movie window, PatBlt black, creates MOVIE_WINDOW
  (FUN_004213e0, class vtable 0x658bb0, wndproc FUN_00421790).
- FUN_004213e0 builds path = registry HKLM\SOFTWARE\LucasArts
  Entertainment Company\Star Wars Rebellion\TooBigLocation + "\" +
  FUN_00421ba0(id).
- WM_CREATE (FUN_00421790): optional caption window (FUN_0041ecf0 at
  x=4,y=404, 580x70, text = param caption, only when non-empty — story
  movies pass the story message's +0x50 text); starts Smacker via
  FUN_00421300 -> FUN_005f6920 (type 1 Smacker FUN_005f6cd0; open check
  FUN_005f71f0 = SmackOpen); sets volume from registry MusicVolume
  (FUN_004213a0).
- Cursor hidden: FUN_00414830 ShowCursor(0) loop; WM_MOUSEMOVE ->
  SetCursor(NULL).

### End / skip
- Natural end: FUN_005f7240 at last frame SendMessage 0x402 -> wndproc
  stops, posts 0x405(wParam 0xf, lParam id) to app, calls callback.
- Esc or LEFT MOUSE held: FUN_005f7240 polls GetAsyncKeyState(VK_ESCAPE)
  | GetAsyncKeyState(VK_LBUTTON) while SmackWait says wait -> SendMessage
  0x404 -> same stop/post 0x405/callback. Applies to every movie incl.
  201/202.
- Any other key (WM_KEYDOWN != 0x1b, disasm 0x421a16..0x421a80): stop +
  post 0x405 + callback, EXCEPT ids 0x70/0x71 (trench 202/201) where it is
  ignored. WM_KEYDOWN Escape itself posts message 0 (no-op) — Esc skip is via
  the async poll.
- WM_COMMAND 0xf1: stop + post 0x405.
- Skipping one movie ends only that movie; the app's 0x405 handler chains.

## Missing media
- Startup FUN_004165b0 -> FUN_00416950: requires TooBigLocation on a fixed
  drive (GetDriveTypeA==3) AND FindFirstFile(<loc>\MDATA\MDATA.000). Else
  FUN_004067e0 MessageBox "Error: CD Not Found" / "The CD-ROM could not be
  found, please check the drive.\nIf the problem persists, please reinstall
  from the original CD." (strings set by FUN_00406600, overridable from
  data0.reb) and the program exits (returns 0, no app window).
- Per-movie: FUN_005f6920 if SmackOpen fails -> FUN_0060a9e0 MessageBeep +
  MessageBox(MB_ICONHAND|MB_SYSTEMMODAL, text=path, caption "PathName
  unknown to VideoManager") then PostMessage 0x403 -> treated as finished
  (stop/post 0x405/callback) so the flow continues.

## App state machine
FUN_00414330(state) (jump tables 0x414694/0x4146c4); FUN_00401040 is the
request wrapper. State -> handler:
1,2,7 -> FUN_00414830 (play movie by state); 3 -> reset game
(FUN_005974c0, FUN_00597050) then FUN_00414a80 = cockpit main menu;
4,5,11,12 -> FUN_00414a80/menu-like; 8 -> FUN_00414eb0(1); 9 -> FUN_00415120;
13 -> quit confirm (FUN_00401110 0x470 ...); 14 -> quit; 16..19 -> tear down
galaxy windows (+0xb8/+0xbc), FUN_00597050, then FUN_00414830.

FUN_00414830 state->movie: 1->0x65(000), 2->0x66(001), 7->0x68(003) if
player side (+0x108)==1 (Alliance) else 0x69(004), 0x10->0x6b(105),
0x11->0x6a(106), 0x12->0x6d(108), 0x13->0x6c(107).

App wndproc FUN_004138a0, 0x405 with wParam 0xf (movie done):
0x65 -> state 2; 0x66,0x6a,0x6b,0x6c,0x6d -> ShowCursor(1) + state 3
(main menu); 0x67 -> state 7; 0x68,0x69 -> state 8 (start game);
other ids (005 credits, 101-104, 201/202) -> post 0x402(lParam id) to the
current panel window (+0xc0) which resumes.

## 1. Intro
First WM_ACTIVATEAPP (FUN_004138a0, DAT_006b1468): state 1 unless
FUN_00597480() (video-manager/lobby flag DAT_006be354; hyp: DirectPlay lobby
launch) -> state 5 (skips intro).
- MDATA.000 (0x65, 17 s, 640x480): LucasArts logo then Coolhand logo (contact
  sheet).
- then MDATA.001 (0x66, 169 s): "A long time ago in a galaxy far, far
  away...." card, STAR WARS title, REBELLION crawl, then opening cinematic
  with opening credits (e.g. "Sound Designer Jeff Kliment"), ending on a
  Lambda shuttle -> main menu (state 3).
All of these are in the two movies; nothing is drawn by code. Port mapping:
000.webm = MDATA.000, 001.webm = MDATA.001. The port plays only 000 then
goes to MainMenu (CutsceneKind::Intro) — 001 is missing from its intro.
Skip: Esc/left-click/any key ends the current movie only; 000 skipped ->
001 starts; skipping 001 -> menu.

## New game
Faction button (FUN_00406000 0x65 Alliance/0x66 Empire) -> state 7 ->
MDATA.003 (Alliance, shuttle to a cloud city) / MDATA.004 (Empire, Star
Destroyer hangar) -> state 8 starts game.

## 2. End of game
FUN_004c0f60/FUN_004c3060 -> FUN_0041d830 -> FUN_00425c90(victory word):
- 0x100000 (Empire win): state 0x11 + (player side +0x9c == 2) ->
  Alliance player 0x11 -> MDATA.106 (Alliance defeat crawl "The Alliance is
  doomed..."); Empire player 0x12 -> MDATA.108 (Empire victory, "This
  insignificant Rebellion has been crushed...").
- 0x1000000 (Alliance win): side==1 -> 0x10 -> MDATA.105 (Alliance victory
  "United in victory..."), else 0x13 -> MDATA.107 (Empire defeat "The
  Emperor and Lord Vader are lost...").
FUN_0042db70 posts 0x482(0,state) to the galaxy window; FUN_00422ce0 waits
for pending animations then FUN_00401040(state). State 16-19 destroys the
game windows, plays the movie; on its end (any skip too) ShowCursor + state
3 -> game reset + main menu. No dialog, no continue/restart option found
(hyp: none exists; only the movie then the cockpit menu).
Port: uses 105-108 as story cutscenes (wrong) and a VictoryModal.

Story movies (0x482 wParam 1, FUN_00425ce0 -> FUN_004019a0(+0x5c,
caption +0x50)):
- 0x6f MDATA.101: system destroyed by the Death Star (FUN_004950d0,
  texts 0x70c9/0x70ca).
- 0x73 MDATA.102: Alliance HQ destroyed (FUN_0048e5a0, 0x7188/0x7189).
- 0x74 MDATA.103: Coruscant captured (FUN_0048f790, 0x7170/0x7171).
- 0x72 MDATA.104: Death Star sabotaged (FUN_0048d470, 0x71bd/0x71be).
Port story_event_to_cutscene maps 101-108 to Luke/Han/Vader events — does
not match the original.

Trench run (FUN_005cfec0): state 6 success -> FUN_005c0fb0 -> FUN_0059cc60
-> 0x71 MDATA.201; state 7 failure -> FUN_005c0ff0 -> FUN_0059cc80 -> 0x70
MDATA.202; guarded by DAT_006bc45c (hyp: local human present); music stopped
(FUN_00610c00(0)); returns to tactical view (0x402 to current window).
Not skippable by ordinary keys; Esc / left click do skip.

## 3. Credits
Main menu FUN_00406000 command 0x73 (LucasArts logo button, rect
459,242,33,28): FUN_00610c00(0) stop music, flags +0x100/+0x104 = 1, hide
menu (vtable+8), FUN_004019a0(0x6e, 0, LAB_00406520) -> MDATA.005 (2:00,
credits roll over cinematic footage: "Technical Director Hugh Sider",
Programming, voice cast, Manual, sales...). No TEXTSTRA/RCDATA credits text
and no scrolling code: the credits are the movie. Leave by Esc / left click
/ any key or natural end. Callback LAB_00406520 (0x406520..0x4065f5):
repaints the menu window (FUN_005ff440/FUN_005fc3f0/FUN_00600010),
InvalidateRect, show(5), FUN_00401a80/FUN_00401a60, FUN_00610be0(1,0),
FUN_004173c0 (music re-init), FUN_0041d3b0, FUN_00417520 (restart menu
music; hyp: MDATA.300 index 0). Port: draws a scrolling text Credits mode
(draw_credits) — not the original.

## Port deltas (summary)
- Intro: add MDATA.001 after 000.
- Skip: original = Esc, left click, any key (one movie at a time); port =
  Esc/Space.
- Story map 101-104 wrong events; 105-108 are endings, not story.
- Endings: movie then reset to cockpit menu; port shows VictoryModal.
- Credits: MDATA.005 movie; port scrolling text.
- 003/004 new-game movies not used by the port.

## Ported (2026-10-30)

`crates/rebellion-app/src/main.rs` (`CutsceneKind`, `ending_cutscene`,
`new_game_cutscene`, `key_skips_cutscene`): the intro plays 000 then 001;
a side's start plays 003 or 004 before the game; the credits are 005; the
game's end plays 105..108 and returns to the main menu (the replacement
victory modal and scrolling credits are gone). Escape or a left click ends
any movie, any other key every movie but the trench run's. The browser
plays them in the page's `<video>` (`video_player.rs`). The story movies
101..104 followed the same day (see "Story movies (ported)" below).

## Story movies (ported 2026-10-30)

Checked by hand in the disassembly: the four classes' slot `+0x10`
(vtables `0x65b758` → `0x4953a0`, `0x65b450`/`0x65b568`/`0x65b3c8` →
`0x48d640`) return 6; `FUN_0048a060` (`0x48a075..0x48a082`) decrements it
and jumps through `0x48a168`, whose entry 5 is `0x48a149`: `lea ecx,
[edi+0x28]; call 0x5f5440` and return. So these four messages reach only
the movie list: no Message Index row, no alert, no advisor event.

Port (`main.rs`, `message_log.rs`, `story_messages.rs`):
- `MessageLog::queue_story_movie` holds the list (`+0x28`);
  `victory_conditions_movies`, `death_star_sabotaged_movie` and
  `death_star_destroyed_movie` file the movie with its caption (the body)
  where the port used to post a message.
- In the galaxy view, with the clock neither paused nor pausing, the
  oldest movie plays (`CutsceneKind::Story`), one at a time; its end
  returns to the same galaxy view and its music.
- The caption (`draw_story_caption`) is white font 4 at (4, 404), 580 by
  70, in the 640 by 480 movie client, wrapped by words and clipped.
- Every movie is fitted into the 640 by 480 client (`movie_client`), in
  the native player and in the browser's `<video>`, whose bars let the
  canvas show through.
- port: the strategic clock holds during the movie, as in every cutscene;
  hyp: the original's keeps running (nothing on the path pauses it).
- port: a waiting movie is not saved; whether the original saves `+0x28`
  is not read.
- port: the caption field's scroll bar (text taller than 70) is not drawn.


Sources: ~/Games/rebexe-decompile/decompiled-typed, objdump of REBEXE.EXE (GOG),
web/data/base/textstra-messages.json (TEXTSTRA RCDATA). `hyp:` marks inference.

## 0. Object graph

- `DAT_006b14e4` (game, built by `FUN_00435bc0`): `+0xc` = the LOCAL HUMAN player,
  `+0x10` Alliance player, `+0x14` Empire player. The human is a
  `Class_0065ad70` (`FUN_004861b0`, 0xc4 bytes, side at `+0x24`). The other side is
  an AI player (`FUN_004857a0`, 0x42c bytes) in single player (`param_1[7]==1`) or a
  remote player (`FUN_00484e70`, 0x60 bytes) otherwise. Only `Class_0065ad70`
  owns a message manager.
- Message manager M = human `+0x6c` (`M+0` = the viewer side, `param_1[0x1b] =
  param_1[9]` in `FUN_004861b0`). Lists (`FUN_005f5440` add, `FUN_005f54d0`
  remove-by-id `+0x18`, `thunk_FUN_005f5060`/`FUN_00568ee0` = first):
  - `M+0x10`: ordinary messages (kinds 3,4,5,7,8);
  - `M+0x1c`: kinds 1,2 (flags bits 1/2);
  - `M+0x28`: **the story-movie queue** (kind 6);
  - `M+0x8`: flags; bit `0x20000000` = "a story movie is playing".

## 1. The queue: who fills it, conditions, sides

Path (per frame): `Class_0065ad70_vt28` = `FUN_00486520`:
1. `FUN_004befe0` (ecx = human `+0x5c`) pops the next ready notification batch;
   `FUN_00488b50(batch)` (ecx = M):
   - `FUN_00488de0` maps the batch to a message type (first record that maps wins);
   - `FUN_00489740(type, batch)` builds the class, feeds every record
     (`FUN_004974f0` -> record vt+0x1c -> class slots +0x1c/+0x20/+0x24), and
     **keeps it only if `msg+0x40 != 0`** (else deletes it; tail of `FUN_00489740`);
   - `FUN_0048a060(msg)` files it by the class's kind (vt+0x10). Jump table
     `0x48a168`: 1 -> `0x48a089` (+0x1c, flag 1), 2 -> `0x48a0a0` (+0x1c, flag 2),
     3/4/5/8 -> `0x48a0cf` (ordinary: title check `FUN_00583c40`, category mask
     `FUN_0048a1c0`, `M+0x50 |= mask`, message index `FUN_0041db20`, alert
     `FUN_0041d870`), **6 -> `0x48a149`: `FUN_005f5440` on `M+0x28` only**, 7 ->
     `0x48a0b7` (+0x10 and open window `FUN_0041d7d0`). (Ghidra's decompile swaps
     6 and 7.)
2. If human `+0x48 == 0` and `FUN_0041dc10() == 0`: `FUN_00488b90` (ecx = M) —
   if `M+0x28` non-empty and flag `0x20000000` clear: `FUN_0041d850(first)`, set
   the flag (`0x488b90..0x488bcb`).

Kind 6 classes: vt+0x10 returns 6 (`mov eax,6`) for exactly these four:

| Movie | Class vtable | vt+0x10 | ctor (type from `FUN_00488de0`) | builder |
|---|---|---|---|---|
| 0x6f MDATA.101 | `0x65b758` | `0x4953a0` | `FUN_00494f80` (type 0x15) | `FUN_004950d0` (slot +0x20), `FUN_004952a0` (+0x24) |
| 0x73 MDATA.102 | `0x65b450` | `0x48d640` | `FUN_0048e500` (type 0x1f) | `FUN_0048e5a0` (+0x20) |
| 0x74 MDATA.103 | `0x65b568` | `0x48d640` | `FUN_0048f700` (type 0x1e) | `FUN_0048f790` (+0x20) |
| 0x72 MDATA.104 | `0x65b3c8` | `0x48d640` | `FUN_0048d3e0` (type 0x23) | `FUN_0048d470` (+0x1c) |

Type selection in `FUN_00488de0`:
- `0x302` (Destroyed) whose object is a system (family 0x90..0x97) -> 0x15.
  The batch's `0x14f` (SystemBombard) record with value 0 would map to 0xf (the
  bombardment summary) but is skipped (type 0) when the system's `+0x50 & 8` is
  set (hyp: the destroyed bit), so the Death Star's kill yields only the story
  message, not a bombardment report.
- `0x128` (the victory word): new bit 0 set and old clear -> 0x1f (HQ); else new
  bit 0x100 set and old clear -> 0x1e (Coruscant).
- `0x305` (GameObjDestroyedSabotage) with object family 0x18..0x1b -> 0x23; other
  families -> 0x22 (Saboteurs Strike).

Gates on adding: only `+0x40` (and the type mapping). Kind 6 bypasses the
title check, the category mask / Message Alerts settings (`FUN_0048a1c0`), the
message index and the alert. No side test in the four builders:

| Movie | `+0x40` set when | Viewer-specific? |
|---|---|---|
| 101 | `0x182` (FleetBombard): the fleet resolves in the viewer's table (`FUN_004f36a0` family 8..15); also `0x14f` with record `+0x28 == 0` and the system resolves (`FUN_004952a0`) | no; both sides |
| 102 | `0x128` with value bit 0 (`FUN_0048e5a0`) | no; both |
| 103 | `0x128`, Coruscant `0x90000109` resolves and its `+0x24 & 0xc0 == 0x40` (Alliance) (`FUN_0048f790`) | no; both |
| 104 | `0x305`, destroyed object family 0x18..0x1b and the second key's container (`+0x1c`) is a system 0x90..0x97 (`FUN_0048d470`) | no; both |

So whichever side the local human plays sees the movie. The AI/remote side has
no manager, so nothing is played "for" it. hyp: in multiplayer each machine's
local human gets its own movie.

## 2. End of movie, removal, what follows

Movie end (natural or skip) -> app `FUN_004138a0` `0x405` wParam 0xf: destroys
`+0xc4` (movie window); ids 0x6f/0x72/0x73/0x74 are not in the switch, so the
default (`0x413e6c..0x413ee8` in the decompile): if `+0xc0` (the active panel,
the galaxy window) exists: GetDC, re-realize the palette if `+0xc0` is `+0xb8`
or `+0xbc` (`FUN_005ff440`, `FUN_005fc3f0`, `FUN_00600010`), `ReleaseCapture`,
`+0xc0` vt+4(5) = `FUN_005ffe40` -> `ShowWindow(SW_SHOW)`, then
`PostMessage(galaxy, 0x402, 0, id)`.

Galaxy window `FUN_00422ce0` `0x402` (line ~747): if `+0x498` (the message saved
by `FUN_00425ce0`) is set:
1. `FUN_0041d330(msg)` -> `FUN_00436920` (game `DAT_006b14e4`):
   - `FUN_00488ca0(msg)` on human `+0x6c`: kind 6 -> remove from `M+0x28`
     (`FUN_005f54d0(msg+0x18)`), **clear `0x20000000`**, delete the message (vt0(1)).
     (`0x488ca0..0x488ce3`.)
   - Alliance and Empire players vt+0x17c (`Class_0065ad70_vt17c` = `FUN_004883d0`
     -> `FUN_00486fb0(5, ...)` -> default `FUN_00487c50(5, ...)`: builds order kind
     5 via `FUN_004f5cd0`; purpose untraced, hyp: a "story shown" acknowledgement).
2. `+0x498 = 0`.
3. `FUN_00401a60(FUN_00401a80())`: re-apply the master Volume (`FUN_00416360`).
4. `FUN_00610be0(..., 1, 0)`, `FUN_004173c0` (music re-init: MDATA path,
   MusicVolume, MusicSwitch), `+0xb4 = 1`, `FUN_00422cb0` -> `FUN_00417520(
   FUN_0041d3b0())`: **galaxy music restarts**.

Nothing opens and no message is filed. The story message is **never** an ordinary
message or alert, before or after: kind 6 skips `M+0x10`, `FUN_0041db20` (index),
`FUN_0041d870` (alert), and the message object is deleted at the movie's end.
The caption under the movie is its only text. The next queued story movie starts
on the next frame's drain.

## 3. Clock, galaxy window, return

- Start: `FUN_00425ce0` stores msg at galaxy `+0x498`, posts `0x482`(1, msg).
  `0x482` handler (line 1174): returns at once if `FUN_004fcee0() < 2` (scheduler
  sub-tick count < 2); posts `0x409` to itself; hides `+0x118` (`param_1[0x46]`,
  vt+8); pumps messages and ticks (`DAT_006b14cc` vt+0x2c, `FUN_00401bd0`) until
  `FUN_00601340()` (animations done; flag `+0xa4 = 1` meanwhile); `SetCapture`
  (`FUN_00601080`); `FUN_00610c00(0)` stop music; `FUN_004019a0(msg+0x5c,
  msg+0x50, 0)`.
- `FUN_004146e0`: the active panel `+0xc0` gets vt+8 = `FUN_00405490` ->
  `ShowWindow(SW_HIDE)`. **Hidden, not destroyed**; `+0xc0/+0xb8` stay. A previous
  movie window is destroyed, the client PatBlt black, `MOVIE_WINDOW` created
  (`FUN_004213e0`, style `WS_CHILD`, black brush), shown, focused.
- Contrast endings (states 16-19, `FUN_00414830`): `+0xc0` hidden, `+0xb8/+0xbc/
  +0xc0` nulled, `0x469` posted (tear-down), cursor hidden; return -> main menu.
- Clock: no pause call on this path (`FUN_00425ce0`, `0x482`, `FUN_004146e0`,
  `FUN_004213e0`, WM_CREATE `FUN_00421790` never call `FUN_0041d310`/
  `FUN_00436890`, the pause that the Pause command `0x46a/0x46c` uses in
  `FUN_00415ec0`). The Smacker player runs on its own thread (`FUN_005f7020`
  CreateThread `FUN_005f73f0`), and the app loop `FUN_00413560` keeps calling
  `FUN_00401bd0` -> `FUN_0040a050` (the frame scheduler). hyp: **the game clock keeps
  running during a story movie**. (Only the next story movie is held back by
  `0x20000000`.)
- Return: as in section 2: galaxy shown again (same view state), capture
  released, music restarted. No dialog.

## 4. Caption window

`FUN_00421790` WM_CREATE (when the caption string `+0xa8` is non-empty; the
caption is `msg+0x50`, set by `FUN_004213e0` `FUN_005f3090(param_10)`):
- back buffer: `FUN_005fbd20(FUN_006037f0(0xc), 0x2b35, 2)` (bitmap 11061, used for
  its palette, hyp), `FUN_005fbda0(w, h, bmp)` -> `+0xb0`, filled with index 0
  (`FUN_005fcfe0(0,0,w,h,0)`): black.
- `FUN_0041ecf0(module, 4, 0x194, 0x244, 0x46, movieWnd, backbuf, 0)`: class
  "TextScrollField" (vtable `0x658ad0`, wndproc `FUN_0041efe0`), style
  `0x56000000` (WS_CHILD|WS_VISIBLE|WS_CLIPSIBLINGS|WS_CLIPCHILDREN), x=4 y=404
  580x70. Own 580x70 buffer from the parent's (`FUN_005fbda0`, `FUN_005fcc30(buf,
  0,0,4,404,0,0)`: copies the parent's black at (4,404)). Left margin `+0x12c = 0`,
  wrap width `+0x130 = 580`, no caret (`FUN_0041fb50(0)`), defaults font 4.
- After: `+0x24 = 0xf2`, `+0xb4 = 0x02ffffff` (PALETTERGB white),
  `FUN_00420550(4)` -> font 4 (`FUN_0060eed0`: Arial, lfHeight 16, weight 400,
  quality 2), set text, `FUN_0041fc30` layout, create, add child.
- Layout `FUN_0041fd00`: `GetTextMetricsA` -> line height `+0xc8` = tmHeight;
  `DrawTextA(hdc, text, -1, rect 0,0,580,70, 0x2410 = DT_CALCRECT|DT_WORDBREAK|
  DT_EDITCONTROL)`; if taller than 70 a scroll bar is added (`+0x98`) and the wrap
  width shrinks by its width.
- Paint `FUN_0041efe0` WM_PAINT: `SelectPalette(DAT_006be5a0)`,
  `FUN_005fc140(hdc, ..., SRCCOPY)` (blit the black buffer), `FUN_0041fe70`:
  `SelectObject(font)`, `SetTextColor(0x02ffffff)`, `SetBkMode(TRANSPARENT)`,
  then word by word `GetTextExtentPoint32A` + `TextOutA(hdc, x, y, word, n)`,
  x from 0, y from 0, new line at '\n', at a space overflowing 580, tabs to
  `+0xdc*4`; stops when `y + tmHeight > 70`. **Left-aligned, top-aligned,
  word-wrapped, white Arial 16 on black.**
- The movie is SmackToBuffer-centred ((640-640)/2, (480-324)/2 = 78): rows
  78..401. The caption child (404..473) is below it, not over it; the movie window
  has no WS_CLIPCHILDREN but the regions do not overlap.

## 5. Texts (TEXTSTRA RCDATA, `FUN_0060b9d0(dst, id, a1, a2, a3, a4)`)

| id | text | args |
|---|---|---|
| 0x70c8 (28872) | "{2:1} Destroyed!" | a1 = DS unit (`+0x68`), a2 = system (`+0x60`) |
| 0x70c9 (28873) | "The Death Star has destroyed the {1:4} system {3:1}." | a1 = Side(`+0x64`), a2 = DS unit, a3 = system |
| 0x70ca (28874) | "The Death Star has destroyed the neutral system {2:1}." | a1 = DS unit, a2 = system |
| 0x7188 (29064) | "Alliance Headquarters Destroyed" | none |
| 0x7189 (29065) | "The Empire has destroyed the Alliance Headquarters." | none |
| 0x7170 (29040) | "Coruscant Captured" | none |
| 0x7171 (29041) | "The Alliance has seized the Imperial capital system of Coruscant." | none |
| 0x71bd (29117) | "Death Star Sabotaged" | none |
| 0x71be (29118) | "The Rebel Alliance has sabotaged the Death Star at {1:1}." | a1 = the system holding the DS's fleet |

`:1` = name, `:4` = side adjective ("Alliance"/"Imperial", `0x4d70`/`0x4d71`).
The caption is the body (`+0x50`), not the title (`+0x44`).

`FUN_004950d0` (disasm `0x4950d0..0x495277`):
- `0x182`: fleet = `FUN_004f36a0(viewer, key)`; iterate its units of family
  0x18..0x1b (`FUN_005003d0(fleet, 3)`), first one's key -> `+0x68`; `+0x40 = 1`.
- `0x14f` (`FUN_004952a0`; the record calls slot +0x24 with (code, &key, rec+0x28,
  rec+0x24), `FUN_0043e980`; `FUN_0053fcf0` stores the system's side `KeyAndSide>>6
  &3` at rec+0x24 and the bombard value at rec+0x28): system key -> `+0x60`; value
  0 -> `+0x40 = 1`; else `+0x64 = system's side`.
- notification 2: `+0x64` ("param_1+100") is the destroyed system's owner (as
  reported by the bombard notification): 1 -> Side(1), 2 -> Side(2), else none.
  Side -> `0x70c9` ("the Alliance/Imperial system X"), none -> `0x70ca` ("neutral
  system X"). It does not depend on the viewer: both sides read the same caption.
  `+0x5c = 0x6f`. (A dead `FUN_004f3dd0(viewer, 2)` call precedes.)

## 6. What can prevent or delay the movie

- Not the local human's message (no manager): never (`FUN_00435bc0`).
- `+0x40` not set (section 1 table) -> message deleted in `FUN_00489740`.
- Drain gated per frame (`FUN_00486520`): human `+0x48 != 0` (a hold-at-tick
  request, `Class_0065ad70_vt174` via `FUN_00436890`), or `FUN_0041dc10() != 0`:
  no scheduler `DAT_006b1218`, or it is held at the current sub-tick
  (`FUN_0041e300`: the Pause state, set by `FUN_0041d310`). So **no story movie
  starts while paused**; it waits in the queue.
- One at a time (`0x20000000`).
- `FUN_0041d850`: no galaxy window (`DAT_006b14cc`) -> nothing posted, but
  `FUN_00488b90` sets the flag anyway. hyp: the queue then stalls.
- `0x482` handler: `FUN_004fcee0() < 2` -> returns without playing; `+0x498` stays
  set and the flag stays. hyp: also a stall (only at the very first sub-ticks).
- `FUN_004019a0`: app `DAT_006be3b8` null.
- Missing MDATA file: MessageBox "PathName unknown to VideoManager", then treated
  as finished (`0x403`).
- No options flag, Message Alerts setting, or multiplayer test on this path (none
  found in `FUN_0048a060` kind 6, `FUN_00488b90`, `FUN_00422ce0 0x482`,
  `FUN_004146e0`, `FUN_004213e0`). MusicSwitch/MusicVolume affect only audio.
