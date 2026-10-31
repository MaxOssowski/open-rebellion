# Keyed bitmap blits

How the original skips a bitmap's transparent pixels, recovered from the
REBEXE decompile corpus (2026-10-31).

## Key selection

`FUN_005fd170` (8-bit surfaces) stores the key in the bitmap object's byte
`+0x08`: the first byte of the DIB pixel data, which is the bottom-left pixel's
palette index for a bottom-up DIB. Two exceptions read the RLE stream instead
when the header's compression field is 1. On a 24-bit surface it stores the
first pixel's three bytes at `+0x09..+0x0b`.

## Comparison

`FUN_005fd0f0` passes that key to `FUN_005fc710` on 8-bit surfaces (or to
`FUN_0060e710` with the 24-bit key). The copy loop in `FUN_005fc710` writes a
source byte only when `*src != (char)key`: one exact index, no colour range.
`FUN_00602d30` (CoolStrobeButton art) blits through the same path.

## Consequence for the port

The port keyed its "blue screen" resources with a range (`r < 32, g < 32,
b > 192`), which also removed the palette's near blues (0, 0, 196),
(0, 0, 223) and (28, 28, 210). 33 of 1183 blue-keyed resources hold such
pixels (STRATEGY 900, 901, 6800, 9006, 9010, 10372, 10373, 10378, 10379,
10711, 10872..10875, 10951, 10953; GOKRES 1092, 1155, 2704, 5697, 5761, 5763,
5957, 9857..9859, 10062, 16386, 19478, 22144, 22147, 23574, 26243). A0 draws
them:

- `.artifacts/a0-vm/status/01.png`: 25 of 901's 48 near-blue pixels equal the
  bitmap; the other 23 lie under the droid and control animations.
- `.artifacts/a0-vm/encyclopedia/index-empire-day143.png`: the Empire rail
  button 10378 at (426, 143) shows its (0, 0, 196) and (28, 28, 210) pixels.

The port now keys exactly (0, 0, 255). Every surveyed key in this set is that
colour except 10873 and 10951, whose first pixel is (35, 36, 35); they keep
the blue rule pending an A0 capture of the pressed posting toggle.

## Buttons the port drew opaque

The Encyclopedia index's category buttons 10339..10344 and 11615..11618 and
the Empire's 0x67 rail button 10380/10381 are CoolStrobeButton art with a blue
first pixel, but the port drew them unkeyed. A0
`encyclopedia/index-empire-day143.png` shows their corners keyed: 0x6f's
bottom-left (36, 118) shows the content pane's white, and 10380's corners at
(426, 89) and (426, 126) show the rail. They now key blue too.
