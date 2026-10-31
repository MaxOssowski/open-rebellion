# The main menu's states against A0 (PRE-02)

Date: 2026-10-31.

## A0

From the VM's Empire game, Game Options' Restart (and Yes) returned to the
main menu (`.artifacts/a0-vm/main-menu-states`, hashes in
`a0-captures.sha256`). The session then made these clicks, capturing a
frame after each:

1. the galaxy lever three times;
2. the game-type switch twice;
3. Intermediate, Expert, then Easy;
4. the Empire button held, captured, and released off the button.

The menu was still there after the release (`11-after-cancel`).

## Lever finding

After the third click the lever showed what it had shown after the first.
`Class_0066dfc8_vt24` keeps a position at `+0x134` (2 Standard, 1 Large, 0
Huge; command 0x6a, `FUN_00406000`) and a direction at `+0x138`. Each click
steps the position one way. Past either end the lever turns back to 1 and
reverses. From Standard it therefore runs Large, Huge, Large, Standard,
Large and so on. The port wrapped round instead (Huge, then Standard). It
now keeps the direction (`MainMenuState::lever_returning`,
`next_lever_position`), with the unit test
`the_galaxy_lever_swings_to_huge_and_back`.

## Gate

`tools/interface-parity/main-menu-states.mjs` replays the session on Linux
Chromium and compares each step with its A0 frame
(`main-menu-states-result.json`). Every step passes:
- **Animation.** The buttons animate always, so only the regions that do
  not animate are compared.
- **Exact regions.** The lever, the three size lights and the selected
  difficulty (its pressed art) match pixel for pixel at every step.
- **Game-type label.** Its ink box for "Standard Game" and "Headquarters
  Victory" is within 1 pixel at the left, top and bottom. The width is
  within the accepted text deviation.
- **Empire held.** The held Empire button matches A0's pressed art, leaving
  out A0's pointer.
- **Release off the button.** Nothing starts.

`main-menu-rest.mjs` still passes.
