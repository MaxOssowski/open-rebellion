# The Assault Summary's Display buttons pressed in the browser (2026-11-06)

This covers EVT-03-C014 ("go to system") and EVT-03-C015 ("go to fleet").

## What the gate does

`tools/interface-parity/fleet-window.mjs` runs on Linux Chromium with WASM sha256 `b91b95c48484b2d47788dbd3231ad78111c9c7004169d960e21f08b2776d32cc`.

**bombardment**, both sides:
- After General Bombardment the gate opens Display (`0x68`) from the rail.
- It then presses `0xc` "Fleet" at window (226, 182), 169 by 96 (`FUN_0044c630`).
- The summary closes, and the system's Fleet window opens (`FUN_0045c8e0`, kind 4); before this the gate asserts that no Fleet window was open.

**assault**, both sides:
- After the tip probes, the gate opens Display and presses `0xb`, the system's name at (29, 182).
- The summary closes, and no Fleet window opens (`FUN_00429ce0` alone).

The sector window for the system is open already in both fixtures, so going there shows no new window.

## A0

There is no A0 capture of the Assault Summary's Display page.

The VM's DS_flip_allian game (Alliance) ran on to day 297 and raised "Battle at Yavin". Resolved there, it gave the Battle Summary's Display page (`.artifacts/a0-vm/battle-display/`). That window is `FUN_0046f360`, not the Assault Summary, so it does not settle these two cells. In that window:
- the Fleet button sits at x 229;
- the labels sit under the buttons;
- the title and the outcome line stay at the top.

That save builds its messages for the Empire behind an Alliance cockpit: the page shows the winner's text `0x7012` and both buttons, although the Alliance fleet was destroyed. All of this matches `ghidra/notes/battle-summary-window.md` and the port's `battle_display_page`. The Assault Summary (`FUN_0044c630`) places the Fleet button at x 226 and centres each label in its button.

## Screenshots (sha256)

- `.artifacts/interface-parity/fleet-window-2026-11-06T16-21-14-671Z-1192041/alliance-bombardment/bombardment-display.png`: `1d02932a59e3741230e70a569c1cb4740caf06fbcb1988c4a05d122ca05743d5`
- `.artifacts/interface-parity/fleet-window-2026-11-06T16-21-14-671Z-1192041/alliance-bombardment/bombardment-done.png`: `c365e723a2c205f0e85f754d9f1d16091bb0e690e421d8bfb94f3f6c2935cb85`
- `.artifacts/interface-parity/fleet-window-2026-11-06T16-21-19-395Z-1192284/alliance-assault/assault-display.png`: `f80a5523aa98b7f7fe0a3d261f2395f6f74d945091391bede49a47f78cccd915`
- `.artifacts/interface-parity/fleet-window-2026-11-06T16-21-19-395Z-1192284/alliance-assault/assault-done.png`: `bcc2b38d30154eb6f3c922e124ba46e366c7bf98fd338decd88904afeda23854`
- `.artifacts/interface-parity/fleet-window-2026-11-06T16-24-32-418Z-1196026/empire-bombardment/bombardment-display.png`: `0732ee09f9bf9d91464e3da1e59e708315775b0297e2a2410b94db44fca71469`
- `.artifacts/interface-parity/fleet-window-2026-11-06T16-24-32-418Z-1196026/empire-bombardment/bombardment-done.png`: `9163bd7e4f97164f636e4a1876f5eaa16f9946474a6924620807df3d9cb66ba3`
- `.artifacts/interface-parity/fleet-window-2026-11-06T16-24-37-103Z-1196286/empire-assault/assault-display.png`: `9020b2df7cf716f93955ba10db569a6d8f42e1068221371af7ce324359ff2f0b`
- `.artifacts/interface-parity/fleet-window-2026-11-06T16-24-37-103Z-1196286/empire-assault/assault-done.png`: `6491f5aac5154a3d60ec62b98a14e1013bc18bf33f1a8d2c63a806d4a7e1c4e1`
- `02-display.png` (A0 Battle Summary Display): `e6fb16f1d099a7957b6bef7a9183e442a84a7d503fbac056a541a0b479a27222`
- `01-summary.png` (A0 Battle Summary): `d13dc6ffd4b6848bd6dfea5f2d7446037feff874e258794d4beb85b879af0028`
- `00-alert-yavin-day297.png` (A0 Battle Alert): `17383352e6b78b94f7bc4420a5d11ae1ccffade19c3af432570be79a2d169f81`
