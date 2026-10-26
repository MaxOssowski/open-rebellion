# Hulls reach the 8-bit screen through the RGB device's colour cube (P24)

Date: 2026-10-26. This settles the renderer behind A0, the main open
point of `../2026-10-25-battle-palette-output/`.

Sources were read statically:

- REBEXE.EXE;
- the game's own `d3drm.dll` (SHA-256 `e04e3781…d4d6`);
- the A0 guest's `C:\Windows\SysWOW64\d3dim.dll` (Windows 10 19045, SHA-256
  `adba7bd0…`). It was read from the VM disk with a read-only export while
  the VM was off;
- the DDrawCompat v0.6.0 source. The game's `ddraw.dll` is DDrawCompat
  v0.6.0, SHA-256 `32d86c43…`.

None of these files is in the repository.

## Which device draws A0

- **REBEXE asks for Ramp by GUID.** `FUN_005d6e10` calls
  `IDirect3D2::CreateDevice` directly:
  - first the HAL GUID (`DAT_0066e468`), only when the wrapper's `+0x120`
    is set;
  - otherwise, or if that fails, `IID_IDirect3DRampDevice`
    (`DAT_0066e458`, F2086B20…);
  - it then calls `IDirect3DRM2::CreateDeviceFromD3D`.

  The device sort in `0x005d6b40` only decides whether a device exists.
- **Windows 10's d3dim.dll still has Ramp.** It holds the Ramp GUID, "Ramp
  Emulation", and a Ramp provider (vtable `0x74001528`, caps
  `dcmColorModel` 1).
- **GOG's DDrawCompat replaces it.** Its `replaceDevice()` (`Direct3d.cpp`)
  turns Ramp, MMX and RGB requests into RGB (`SoftwareDevice` = `rgb`).
  HAL cannot take the 8-bit back buffer: P8 is only `OFFSCREENPLAIN`.
- **d3drm then works in RGB.** `FUN_6d81d5cd` sets the colour model at
  `+0x6c` from the device's caps: 2, RGB.

So the GOG build on Windows 10, which is the build A0 was taken from, draws
hulls with d3dim's RGB emulation into an 8-bit surface. Ramp's texture
colour and shade reduction never applies there. A 1998 machine with the
original DirectX would have used Ramp; that path is not modelled.

## The RGB device's 8-bit output

d3dim.dll:

- `FUN_7402b5dc`, target type 7, selects the writer `FUN_7403e4f0`. The
  dithered entry is the same function, so an 8-bit target never dithers.
- The writer computes, for each channel, `(c × 5 + 128) >> 8`, and writes
  `table[r × 36 + g × 6 + b]`.
- `FUN_74029f67` fills the 216 cells from the cube colours `level × 51`
  (`0x74003168`). Each cell takes the palette's nearest entry
  (`FUN_7402aba0`): squared RGB distance, the lowest index on ties.
- Palette entries flagged read-only (`peFlags & 0xC0 == 0x40`) stay fixed.
  The battle palette's flags `0x44` make all 256 entries fixed.

Against A0 TAC-01-C003: every colour of the Victory (11) and of the
transport (10), in tight boxes, is a cube output. So is every colour of the
space area. Only 99 of the 256 palette entries can come out of the cube.

## The port

- `original_rgb_cube_cell` and `original_rgb_cube` (tactical_assets.rs)
  implement the writer and the table.
- The hull material carries the table as a 216 × 1 texture of palette
  colours (`CubeLut`). It is built from the battle's palette whenever the
  material is built, and kept alive with it.
- The fragment shader (GLSL and Metal) clamps the lit texel, takes its
  8-bit channels, then the cell, then the cell's palette colour. It does
  not dither.

## Against A0

Scenario 37 (`production-hull-scale-a0`), 640 × 480. The boxes are the
same as in `../2026-10-25-side-facing/`, and pixels are counted when their
channel sum exceeds 40.

| | Victory colours (outside the cube) | Transport colours (outside the cube) |
|---|---|---|
| A0 TAC-01-C003 | 12 (0) | 22 (11) |
| Port before | 110 (107) | 211 (207) |
| Port now | 53 (40) | 78 (69) |

What is left outside the cube:

- The blue field lines cross both boxes, drawn antialiased: (0, 0, 64),
  (0, 0, 192) and others. A0's lines are solid palette colours.
- Edge blends. The browser's WebGL context is created with the default
  `antialias: true` (`web/gl.js`), so hull edges mix with the background.
  Native macroquad uses no multisampling.

The transport box in A0 also takes in non-hull pixels.

## Still open

- The browser canvas's multisampling: the next stage.
- Flat shading. d3drm's device default quality is `D3DRMRENDER_FLAT`
  (`0x6d81a08b`, `+0x68` = 0x88), and REBEXE sets no device quality.
  `FUN_6d827ec2` takes the lower of device and mesh quality, so hulls are lit
  once per face. The port still interpolates per vertex.

## Tests

- `the_rgb_device_maps_colour_through_the_six_level_cube`:
  - the cell formula at its edges (25, 26, 77, 128);
  - the table's nearest entry, its lowest-index tie, and a green level.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2837 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T01-56-40-502Z-3064308`) |
| Replay equivalence | passed |
| `cargo mutants` | 40 tested: 34 caught, 2 unviable, 4 missed |
| Release WASM SHA-256 | `3a8181f6a307309e6bd08fff89f82127bb87f18176c80da3af00aa0b41157e5c` |

The first matrix run failed 136 of 148 scenarios on "glBindTexture called
with an already deleted texture". The cube texture had been dropped once
it was set. The material now keeps it alive.

The 4 missed mutants are GPU paths that only the browser matrix exercises:

- `set_palette_selector`, `load_proof` and `load_participant_family`
  replaced with no-ops;
- the material's `textures` list deleted.

A missing or deleted `CubeLut` fails every tactical scenario's console
check, as the first run showed.
