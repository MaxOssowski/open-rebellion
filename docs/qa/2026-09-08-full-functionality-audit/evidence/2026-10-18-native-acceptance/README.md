# Native GUI acceptance for F-001, F-020, F-029 and F-037

Date: 2026-10-18. These are agent checks, not human ones.

## Setup

- Machine: CachyOS Linux, Hyprland (Wayland with XWayland, `DISPLAY=:1`).
- Driver: `cua-driver` 0.31.0 (x86_64-linux), daemon in its standard
  permission mode. Clicks and keys go to the window in the background
  through X11 `XSendEvent`; the hardware pointer never moved.
- Game: `target/release/open-rebellion`, SHA-256
  `17345ac9f0c6ded888e79b0fab8eab1e21325197ad53f1b53315d29099e47d7c`,
  built from `dbbf71e9` (save v75). It was launched muted with
  `scripts/launch-native.sh`, seed 42 where a script starts the campaign.
  Each run's `binary.txt` records the pid and window id.
- The window was 948 by 1026 with the 640 by 480 canvas scaled by 1.48125
  and centred vertically (offset 157.5). On 2026-10-14 it was 950 by 514,
  so `nat.sh` now takes `SCALE`, `OX` and `OY`. These runs pass pixel
  coordinates (`SCALE=1 OX=0 OY=0`).
- The existing saves (`target/release/saves/0.reb`) were backed up first
  and match the backup afterwards. The test mods and `mods/config.toml`
  were removed afterwards.
- The screenshots show original artwork, so they are not committed. Their
  hashes are in `screenshots.sha256` (kept in
  `.artifacts/native-checks/2026-10-18-native-acceptance/`).

## F-001: save, restart, load, delete

`f001-save/`:
- `start-advance.txt` starts an Alliance campaign and advances 100 days.
- The advance stopped at the Battle at Hoth alert (day 46). Auto-resolve
  (the middle button) settled it, and the result window closed.
- At day 100 (resources 65 / 35 / 37), S opened the Save Game panel.
- Slot [2] was named "native" and saved:
  - `save_state_fingerprint slot=1 tick=100 fingerprint=v1:60b8fee5fbe796af`;
  - `saves/1.reb` (66,660 bytes) appeared on disk.
- The game was then closed.

`f001-load/`:
- A new process (no script) opened the main menu cockpit. The disc icon
  opened the original Game Options screen, which lists "native" in its
  second row. Its Load control (the right icon) loaded it:
  - `load_state_fingerprint slot=1 tick=100 fingerprint=v1:60b8fee5fbe796af`;
  - `[campaign] restored slot=1 mode=Galaxy tick=100` with the same
    fingerprint;
  - the galaxy shows day 100 and 65 / 35 / 37.
- S opened the panel again, and Delete under slot [2] removed it:
  - the slot reads empty;
  - `saves/1.reb` is gone and `0.reb` is untouched.
- The old `0.reb` (an older save version) is listed as "Unreadable save",
  as the no-migration rule intends.

## F-020: mod dependency errors

`f020/` holds three temporary mods (`zz-native-*.mod.toml`), all enabled in
`config.toml`:
- one requires a mod that is not installed;
- one requires `zz-native-base >=2.0.0`;
- the base mod is at 1.0.0.

Results:
- The game log prints `Discovered 3 mods (3 enabled)` and the
  missing-dependency load-order error.
- Tab opened the Mod Manager. The two broken mods are red. Selecting each
  shows:
  - "Error: requires 'zz-native-absent', which is not installed and
    enabled";
  - "Error: requires 'zz-native-base' >=2.0.0, but 1.0.0 is enabled".
- Unticking the base mod wrote `config-after-toggle.toml` to disk. The
  second mod then reads "Error: requires 'zz-native-base', which is not
  installed and enabled": a disabled dependency counts as missing.

## F-029 and F-037: a 1000-day native campaign

`f029-f037/`:
- The script starts an Alliance campaign, turns on Dual AI, and runs 1000
  lines of "Advance 1 tick". "Export Game Log" follows every 200th line
  (`campaign-1000-exports.README`).
- A script line waits while a battle alert is up. `battle-loop.sh`
  auto-resolved the one alert that came (Hoth, day 46) and closed its
  result.
- The game log reaches `[ai] day=1000`.
- The message log keeps 500 messages, so the exports cover days 71 to 400,
  482 to 798 and 765 to 1000. `messages-summary.json` merges them (1350
  messages):
  - informants (notification `0x153`, timer `0x38e`): days 286 (Svivren),
    520 (Corfai) and 747 (Praesitlyn);
  - New Resources (`0x155`, event `0x390`): day 776 (Jerijador, energy);
  - 22 research level advances (timer `0x381` and the research missions);
  - 14 autoscrap refunds (timer `0x382`);
  - 14 character injuries.
- The first try (`f029-f037-run1`, not committed) exported once, at day
  999. Its 500 messages covered only days 754 to 999 and held no incident
  message. That is why this run exports every 200 days.

## Observations

- Launch 5 of 6 stopped on a black intro screen ("SPACE / ESC to skip")
  although `OPEN_REBELLION_MUTE=1` skips cutscenes elsewhere; Space went
  on. The other launches went straight to the menu.
- The Save Game panel shows the save time in UTC ("08:34" for a save made
  at 10:34 local time).
- The in-game panel deletes a slot at once, with no confirmation. The
  Game Options code has a confirmation text for a delete ("Delete the
  selected saved game?"); whether that screen offers a delete was not
  checked.
- The Mod Manager window moved between some clicks. One click then landed
  on the galaxy and opened a sector window. The cause was not traced.

Hashes of the committed files are in `SHA256SUMS`.
