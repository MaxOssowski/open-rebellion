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
plays them in the page's `<video>` (`video_player.rs`). Not yet ported: the
story movies 101..104 (their events and caption box); the misassigned
101..108 story triggers were removed.
