# Tooltips, stage 10: string modules, the main menu, Game Options and the Yes/No box

Date: 2026-10-30. Inventory: `ghidra/notes/tooltips.md` §1.

## String modules

A tool's text comes from a module, `FUN_005fefd0`:
- 1 is TEXTCOMM;
- 2 is TEXTSTRA;
- 3 is TEXTTACT;
- 12 is REBDLOG.

The browser build had only TEXTSTRA's table. Now:
- `dat-dumper --extract-strings` also writes `textcomm.json` (80
  strings), `texttact.json` (149) and `rebdlog.json` (39);
- `build-runtime-pack.py` packs them;
- `main.rs` hands each to `win_tooltip::set_module_strings`. Native
  builds read the DLLs (`rebellion_data::dll_strings`).

The same id can name different texts in different modules (REBDLOG 0x1906
is not TEXTSTRA's "Items to Build"), so `module_string` keys on both.

## The tip's lower limit

The Game Options' bottom buttons, with the pointer at y 402, put their tips
above the pointer at 383 (A0 205–207), although the tip would fit below. So
A0 keeps tips above row 440:
- at 318 the tip goes below (rail-chat, to 358);
- at 402 and at 440 it goes above.

Row 440 is the Windows 10 work area above its 40-pixel taskbar at 640 by
480. `win_tooltip::TIP_BOTTOM` is 440; the hypothesis that the original
follows the player's own work area is noted there. The gates' placement
rule follows it.

## A0 (Windows 10 VM, Empire save, then the main menu)

Captures are in `.artifacts/a0-vm/tooltip/`; hashes are in `a0-captures.sha256`.

| Captures | Screen | What they show |
|---|---|---|
| 200–209 | Game Options (F1) | Save Game, Load Game, Music Volume, Sound Effects Volume, "Restart the Game", Return to the Command Center, Exit; none on Play Music or the tactical toggles |
| 210–212 | the Restart question | REBDLOG "Yes" and "No" |
| 220–234 | the main menu | all eleven buttons and the three galaxy-size rect tools |

## The port

The strings live here:
- `MainMenuControl::tooltip_string` (TEXTCOMM 0x1001..0x100b,
  0x100d..0x100f). The port's own music toggle and Fleet Registry chip have
  none.
- `GameOptionsControl::tooltip_string` (TEXTCOMM).

Both screens now paint the shared tip. The Game Options' question box gives
its Yes/No the REBDLOG tips.

## Browser (Arch Chromium 155, interface WASM `26e61fe7…`)

`tooltip.mjs --a0` passes. On the Empire side it:
- opens the site without a fixture and skips the intro with Escape;
- checks the fourteen main-menu spots against A0 221–234;
- opens Game Options with F1 in the galaxy fixture and checks nine spots
  against A0 201–209, including the two with no tip.

`prepare-site.mjs` now copies the intro movies, so the fixture-free site
requests none in vain.

Every left edge, top and height matches A0. The text-width deviation runs
from about 7% to 20% on these strings: "Select Galaxy Size" is 118 against
98, because A0's system face is narrower than Arial on its capitals.

## Open

The setup screen (§1b, `FUN_0040dfd0`) is the multiplayer and Head to Head
setup, which is out of scope (user decision 2026-10-30, no multiplayer).
Of the message boxes (§1d), only the Yes/No question of Game Options is
ported.
