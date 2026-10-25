# The battle's 8-bit output (P24)

Date: 2026-10-25. This follows the open "8-bit palette output" item of
`../2026-10-25-hull-rendering-a0/`.

## What A0 shows

- A0 TAC-01-C003 (Mon Calamari) has 85 distinct colours in the whole
  frame.
- All of them are entries of the tactical palette resource 5537 (type 303
  in `TACTICAL.DLL`). Every other palette 5531..5557 lacks 3 or 4 of
  them.
- So the battle draws into the 8-bit display through the battle's
  palette. Mon Calamari's palette is 5537.

## How the original picks its renderer

`0x005d6b40` enumerates the Direct3D devices (`EnumDevices`, callback
`0x005d6aa0`):

- It keeps a device only if its `dwDeviceRenderBitDepth` (descriptor
  `+0x9c`, wrapper `+0x114`) holds the display depth. `0x005d6850` maps
  8 bits to `DDBD_8` = `0x800`.
- It sorts them with `0x005eece0`: a hardware device (`+0x144`) first,
  then the mono colour model (`dcmColorModel` = 1, the Ramp driver)
  before RGB.
- It takes the first one (`0x005f5060`). With none it reports "Could not
  find a D3D driver which is compatible with this display depth".

On an 8-bit display this is the Ramp driver wherever one exists.

The game sets texture colours and shades only for its two decals:

- `+0x60`: `SetColors(64)`, `SetShades(1)`;
- `+0x64`: `SetColors(128)`, `SetShades(1)`, with decal size, origin and
  transparency (`0x005d5475`..`0x005d54e1`, `0x005d566d`..`0x005d56c4`).

No call sets the default texture colours or shades
(`IDirect3DRM::SetDefaultTextureColors` / `Shades`). The hull textures
keep Direct3D RM's defaults.

## A test of the simplest model

The port draws in true colour. Its frame of the like-for-like battle
(scenario 37, now at Mon Calamari) was mapped pixel by pixel to the
nearest colour of palette 5537:

| Hull | A0 colours | Port | Port mapped to 5537 |
|---|---|---|---|
| Victory | 12 | 93 | 43 |
| Medium Transport | 23 | 217 | 69 |

The nearest colour alone keeps about four times A0's colours. The Ramp
driver also reduces each texture to a few colours and each colour to a
few shades, then maps them to the palette. Those numbers and the colour
reduction belong to Direct3D RM (`d3drm.dll`) and the Ramp driver, not to
`REBEXE.EXE`.

A0 was taken on Windows 10 through DDrawCompat, where the Ramp driver may
be missing. Then the sort picks an RGB device instead. Which renderer
produced A0 is not settled.

## Changed

- Scenario 37 (`production-hull-scale-a0`) now fights at Mon Calamari, as
  A0 does. Its planet and palette (5537) match A0's; before, it took the
  first empty system (palette 5538).

## Still open

- The 8-bit output itself: the renderer behind A0, and its texture
  colours, shades and palette mapping.

## Probe

`production-death-star-laser-journey` failed now and then at 1280 x 800:
it waited 900 ms of wall-clock time, then captured a moving frame, but
the beam is in view only for its last ~0.15 s. The beam's timer advances
only in unpaused frames (`advance_presentational_effects`). So the probe
now waits for the beam's first draw, pauses after 700 ms, and inspects
still frames, resuming a frame or two at a time until the beam shows.
Three runs of the scenario passed.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2794 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed, with the new probe |
| Replay equivalence | passed |
| `cargo mutants` | no core or render code changed |
