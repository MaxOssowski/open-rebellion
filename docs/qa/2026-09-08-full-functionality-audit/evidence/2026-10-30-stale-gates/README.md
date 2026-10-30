# Stale browser gates after the movie and modality stages

Date: 2026-10-30. A full Linux run of the interface gates, during the
tooltip work, found failures that come from three earlier stages. None of
them comes from the tooltips.

## 1. Story movies in fixtures (522a76c5)

The interface fixtures' galaxies have no Alliance headquarters, so the
victory word's HQ bit (`FUN_005180e0`) queues movie 102 at once. A fixture
that lets days run then played it and covered the screen:
- `fleet-window.mjs` requested `/data/video/102.webm`;
- `personnel-orders.mjs` could not open the speed menu.

**Fix.** In fixture builds, only the StoryMovie scenario plays story
movies (`plays_story_movies` in `main.rs`), as only a fixture-free start
plays the intro.

## 2. The intro on the release site (0899c214, 1886c2be)

The release site now opens on movies 000 and 001. These gates waited 90 s
for the main menu and failed:
- `briefing.mjs`;
- `agent-advice.mjs`;
- `agent-objectives.mjs`;
- `encyclopedia-placement.mjs`.

**Fix.** Each now calls `skipToMainMenu`, which presses Escape while a
`<video>` shows. Escape skips one movie (`cutscenes.mjs`).

## 3. The modal Message Index (66050b2d)

The briefing opens the Message Index on Advice
(`status=opened_by_briefing`). The index is modal, and A0 shows the
keyboard is blocked too:
- `.artifacts/a0-vm/tooltip/170-index.png` is the open index;
- `171-index-f7.png`: F7 opens no Encyclopedia;
- `172-index-f3.png`: F3 opens no Fleet Finder.

`agent-objectives.mjs` and `encyclopedia-placement.mjs` now press Escape
when the briefing opened the index. `agent-advice.mjs` closes the index
before it right-clicks the second droid.

## Result (Arch Chromium 155, Linux copies)

All of these pass after the fixes (`rerun-summary.txt` and the gate runs):
- personnel-orders, briefing and story-movie;
- agent-advice, agent-objectives and encyclopedia-placement.

`fleet-window.mjs`:
- **"destroy system".** This case waited for the Death Star's line as a
  message, but since 522a76c5 it is movie 101's caption (`FUN_004950d0` is
  a story class of kind 6, not a message). In fixtures the movie stays
  queued, so the fleet-load report now also reads its caption
  (`MessageLog::queued_story_movies`), and the case passes.
- **"blockade".** Both sides fail it on the build before the movie,
  modality and tooltip stages too (8c984888, built in a worktree, same
  "no blockade yet"). The failure is older and not part of this work.

These still fail, for older reasons:
- `message-index-shell`: 470 pixels;
- `fleet-move`: the confirmed-move probe.
