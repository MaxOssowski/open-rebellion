# Game Options window: text, save rows, sliders, version label

The control table, switch states, Return art and keying are in
`game-options.md`. The port's gate is `tools/interface-parity/game-options.mjs`.

Read 2026-10-31 from `~/Games/rebexe-decompile/decompiled-typed/` and `objdump`
of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`). All coordinates are window
pixels (the window is the 640x480 COMMON 20002 background). "hyp:" marks a
guess.

Font table (`FUN_0060eed0`, theme.rs `original_font_cell`): font 7 = 18 px
cell bold Arial; font 10 = 14 px cell regular Arial.

COLORREFs carry the `0x02` PALETTERGB flag:
- `0x200ff00` = RGB(0,255,0) bright green
- `0x2008000` = RGB(0,128,0) dark green
- `0x2505050` = RGB(80,80,80) grey
- `0x2ffffff` = white
- `0x2000000` = black

## 0. Correction to existing notes

`ghidra/notes/game-options.md` gives "upstream P58-B12" coordinates. The
constructor `FUN_00407180` has these literal values (`FUN_00602150(module,
x, y, w, h, parent, id, bmp, bmpPressed, bg, 0, 0)`, `FUN_00604cf0(inst, x,
y, w, h, parent, bg, style)`, `FUN_00604570(module, x, y, parent, id,
orient, bg, track, thumb, value, max)`):

| Control | Literal rect / origin | Notes value |
|---|---|---|
| Save button 1-6 | x `0x22`=34, y 81/123/165/207/249/291, 42x20 | (35,y) |
| Name edit 1-6 | x `0x77`=119, same y, `0xa0`x`0x14` = 160x20 | (116,y,162,20) |
| Load button 1-6 | x `0x11f`=287, same y, 41x20 | (285,y) |
| Faction icon 1-6 | (`0x55`=85, y+2), 26x19 bitmap | none |
| Effects slider | origin (`0x188`,`0xbe`) = (392,190) | (391,192) |
| Music slider | origin (`0x188`,`0x82`) = (392,130) | (391,130) |
| Restart | (`0x4c`,`0x17d`) = (76,381), 42x42 | (77,383) |
| Return | (`0xa2`,`0x17e`) = (162,382), 42x42 | (164,383) |
| Exit | (`0xf8`,`0x17d`) = (248,381), 42x42 | (251,383) |
| Play music | (`0x160`,`0x4c`) = (352,76), 19x35 | (351,76) |
| Starfield | (`0x165`,`0x137`) = (357,311), 35x22 | (359,310) |
| Planet | (357,`0x151`=337) | (359,337) |
| Pyrotechnics | (357,`0x16d`=365) | (359,364) |
| High detail | (357,`0x188`=392) | (359,391) |
| Holocube | (357,`0x1a3`=419) | (359,418) |

Quotes from `FUN_00407180`:

```c
piVar8 = (int *)FUN_00602150(uVar5,0x22,*(undefined4 *)((int)local_c8 + local_dc + 0x48),0x2a,0x14,param_1,local_ec + 0x6e,0x273e,0x273f,uVar6,0,0);
iVar7 = FUN_00604cf0(uVar11,0x77,*(undefined4 *)((int)local_c8 + local_dc + 0x18),0xa0,0x14,param_1,uVar6,0);
piVar8 = (int *)FUN_00602150(uVar5,0x11f,*(undefined4 *)((int)local_c8 + local_dc + 0x30),0x29,0x14,param_1,local_ec + 0x78,0x2741,0x2742,uVar6,0,0);
iVar7 = FUN_00604570(uVar5,0x188,0xbe,param_1,0x8e,2,uVar6,0x2745,0x2746,uVar11,100);   // effects
iVar7 = FUN_00604570(uVar5,0x188,0x82,param_1,0x8d,2,uVar6,0x2744,0x2746,uVar11,100);   // music
```

The y tables are `local_c8[6..11]`, `[0xc..0x11]` and `[0x12..0x17]`, each
`0x51,0x7b,0xa5,0xcf,0xf9,0x123`. The icon y table is `local_c8[0..5]`,
`0x53,0x7d,0xa7,0xd1,0xfb,0x125`.

`FUN_00602150` passes x, y, w and h straight to `FUN_005ff910` (no
adjustment): `FUN_005ff910(param_2,s_CoolStrobeButton_006ac7d8,0x46000000,
param_3,param_4,param_5,param_6,...)`.

## 1. Headings: drawn into the background bitmap, not text objects

`FUN_004084b0` starts with:

```c
DAT_006be5ac = FUN_006037f0(1);              // string module (TEXTCOMM) for LoadStringA
FUN_005ff6b0(0x1028,0x3a,0x25,299,0x3a,7,0x200ff00,0x25);
FUN_005ff6b0(0x1026,0x16a,0x25,0x25b,0x3a,7,0x200ff00,0x25);
FUN_005ff6b0(0x1027,0x16b,0x111,0x25a,0x126,7,0x200ff00,0x25);
```

The assembly at `4084d7`-`40850c` shows that `ecx = esi`, the window, and
that a 16-byte RECT is pushed by value as (left, top, right, bottom).

`FUN_005ff6b0` (this = window, `(id, RECT, font, colour, format)`):
1. Runs `FUN_00601060(param_2, local_100, 0xff)`, which is
   `LoadStringA(DAT_006be5ac, ...)`.
2. Draws only when the string is non-empty and the window has a background
   DIB (`+0x98` or `+0x94`).
3. Calls `CreateCompatibleDC`, selects that bitmap (`FUN_005fc130` returns
   bitmap `+0x1c`), the palette `DAT_006be5a0` and font
   `FUN_006002b0(font)`.
4. Draws with `SetTextColor(hdc_00,in_stack_0000001c); SetBkMode(hdc_00,1);
   DrawTextA(hdc_00,local_100,-1,(LPRECT)&stack0x00000008,in_stack_00000020);`.

So the headings are burned into the background surface once, at creation.
They are not the bitmap's own pixels, and they are never repainted as
objects.

| Heading | TEXTCOMM | RECT (l,t,r,b) | Font | Colour | Format |
|---|---|---|---|---|---|
| Saved Games | 0x1028 (4136) | (58,37)-(299,58) | 7 (18 bold) | `0x200ff00` green | 0x25 |
| Sound Options | 0x1026 (4134) | (362,37)-(603,58) | 7 | green | 0x25 |
| Tactical Display Options | 0x1027 (4135) | (363,273)-(602,294) | 7 | green | 0x25 |

Format 0x25 is `DT_CENTER|DT_VCENTER|DT_SINGLELINE`. The text is centred in
the rect: centres x 178.5 / 482.5 / 482.5.

## 2. Text objects (`FUN_00601620`)

The signature is `FUN_00601620(this, strId, x, y, w, h, colour, font,
format, flags)`. It calls:
- `FUN_00601930(x,y,w,h,format,flags)`;
- `FUN_00601c90(colour,0)`, which stores the colour at `+0xc`;
- `FUN_00601c60(font)`, which stores `+0x24` and HFONT `+0x28`;
- `LoadStringA` of `strId` into a string object at `+8`.

`FUN_00601930` stores the rect as left = x, top = y, right = x + w,
bottom = y + h (`param_1[5..8]`), the format at `[4]` and the flags at
`[0xb]`.

The draw is `FUN_00601ce0`:

```c
SetBkMode(param_2,1);
color = SetTextColor(param_2,*(COLORREF *)(param_1 + 0xc));
format = (~*(uint *)(param_1 + 0x2c) & 0x10) << 4 | *(uint *)(param_1 + 0x10);
DrawTextA(param_2,lpchText,iVar2,lprc,format);
```

All objects here pass flags = 3, so bit 0x10 is clear and `DT_NOCLIP`
(0x100) is added. Flags 1 and 2 keep the given width and height
(`FUN_00601b80`). There is no shadow.

| Field | String | x,y | w x h | Font | Colour | Format (effective) |
|---|---|---|---|---|---|---|
| +0xe8 | 0x1025 "Play Music" | (376,79) | 183x33 | 7 | on `0x200ff00` / off `0x2008000` | 0x24\|0x100 = VCENTER\|SINGLELINE\|NOCLIP, left |
| +0xec | 0x101f "Show Starfield" | (394,313) | 183x16 | 7 | flag 0x10000000 | 0x10\|0x100 = WORDBREAK\|NOCLIP, top-left |
| +0xf0 | 0x1020 "Show Planet" | (394,340) | 183x16 | 7 | flag 0x01000000 | same |
| +0xf4 | 0x1021 "Show Pyrotechnics" | (394,367) | 183x16 | 7 | flag 0x00100000 | same |
| +0xf8 | 0x1022 "Use High Detail Models" | (394,394) | 183x16 | 7 | flag 0x00010000 | same |
| +0xfc | 0x1024 "Display Holocube" | (394,421) | 183x16 | 7 | flag 0x02000000 | same |
| +0x100 | 0x1031 "On" / 0x1032 "Off" (music) | (581,79) | 31x33 | 7 | as its flag | 0x24\|0x100, **left-aligned** |
| +0x104..+0x114 | On/Off for the five switches | (577, 313/340/367/394/421) | 31x16 | 7 | as its flag | 0x10\|0x100, **left-aligned** |
| +0x118 | version (section 5) | (25,437) | 310x18 | 10 | `0x2000000` black | 0x25\|0x100 = centred |

The display rows sit at y 313 + 27·k with a 16 px box. Font 7 has an 18 px
cell and NOCLIP, so the glyphs extend 2 px below the box.

`0x1023` "Use 3-D Acceleration" is never created.

Representative quotes from `FUN_004084b0`:

```c
uVar1 = FUN_00601620(0x1025,0x178,0x4f,0xb7,0x21,
        (-(uint)((*(uint *)(param_1 + 0xa0) & 0x100) != 0) & 0x7f00) + 0x2008000,7,0x24,3);
uVar1 = FUN_00601620(0x101f,0x18a,0x139,0xb7,0x10,
        (-(uint)((*(uint *)(param_1 + 0xa0) & 0x10000000) != 0) & 0x7f00) + 0x2008000,7,0x10,3);
uVar3 = *(uint *)(param_1 + 0xa0) & 0x100;
uVar1 = FUN_00601620(0x1032 - (uint)(uVar3 != 0),0x245,0x4f,0x1f,0x21,
        (-(uint)(uVar3 != 0) & 0x7f00) + 0x2008000,7,0x24,3);
uVar1 = FUN_00601620(0x1032 - (uint)(uVar3 != 0),0x241,0x139,0x1f,0x10, ... ,7,0x10,3);
iVar2 = FUN_00601620(0,0x19,0x1b5,0x136,0x12,0x2000000,10,0x25,3);
```

The colour is `0x2008000 + (on ? 0x7f00 : 0)`. On gives `0x200ff00` bright
green and off gives `0x2008000` dark green. **The label and its On/Off use
the same colour.** The decompile has no white for these labels.

A0 `tooltip/200-options.png` (all five on) shows them bright green
`0x00ff00`, and Play Music with its Off dark green `0x008000`. The only
other colour is in tactical states 20-23, where the paint routine
`Class_00658600_vt38` (`FUN_00408bc0`) recolours every display label and its
On/Off to grey before drawing:

```c
if (*(int *)(param_1 + 0xec) != 0) {
  if (bVar1) { FUN_00601c90(0x2505050,param_2); }
  FUN_00601ce0(param_2);
}
```

`bVar1` is `0x14 <= state(+0xa8) <= 0x17`. The music label (+0xe8 / +0x100)
and the version label are never greyed.

### How On/Off and the colours update

`FUN_004082b0` handles command 0x99 (music) and 0xa1-0xa4, 0xa6 (the
switches):
1. It looks up the button (`FUN_00604500`) and reads its toggled state with
   `FUN_00603120(4)`.
2. It sets or clears the mask in `+0xa0` (0x100, 0x10000000, 0x1000000,
   0x100000, 0x10000, 0x2000000).
3. It recolours the label and swaps the On/Off string:

```c
if (iVar3 == 0) {                    // off
  *(uint *)(param_1 + 0xa0) &= ~uVar5;
  FUN_00601c90(0x2008000,0); FUN_00601dc0(hwnd,1);      // label
  param_2 = CONCAT22(DAT_0065d420,0x1032);              // "Off", module word 1
  FUN_00601c90(0x2008000,0);
} else {                             // on
  *(uint *)(param_1 + 0xa0) |= uVar5;
  FUN_00601c90(0x200ff00,0); FUN_00601dc0(hwnd,1);
  param_2 = CONCAT22(DAT_0065d420,0x1031);              // "On"
  FUN_00601c90(0x200ff00,0);
}
FUN_006019a0(&param_2);              // replace the text from {id, module}
FUN_00601dc0(*(undefined4 *)(param_1 + 0x18),1);        // InvalidateRect(rect, TRUE)
```

`DAT_0065d420` = 1 (`.rdata` bytes `01 00 00 00`).

For music (0x99), state 20-23 calls `FUN_00596b70()`. Otherwise it calls
`FUN_00401ab0(~flags & 0x100)` (hyp: starts or stops the music).

## 3. Save rows

Per row k = 1..6 (`FUN_00407180`; the refresh after save/load is
`FUN_00408f10`):

- **Save button**:
  - (34,y,42,20), id `0x6e+k`;
  - bitmaps 0x273e/0x273f (10046/10047), disabled 0x2740 (10048) via
    `FUN_00603150(2,0x2740)`;
  - tooltip 0x1010.
- **Name edit box** `FUN_00604cf0` "CoolStringField":
  - (119,y,160,20), control id `+0x24 = k + 0x82` (0x83-0x88);
  - colours:

    ```c
    *(undefined4 *)(iVar7 + 0xe4) = 0x2ffffff;      // text colour: white
    *(undefined4 *)(*local_e0 + 0xe8) = 0x2000000;  // shadow colour: black
    ```

  - The paint (`Class_0066e038_vt14`, `FUN_00605160`) calls
    `FUN_00600fd0(hdc_00, text, &rect(0,0,w-1,h-1), +0xe4, +0xe8, 0x824,
    +0xe0, -1)`. That function draws the text in `+0xe8` (black), offset
    by `+0xe0` (= 1, the constructor's `param_1[0x38] = 1`), then again in
    `+0xe4` (white) at the rect:

    ```c
    SetTextColor(param_1,param_5); SetBkMode(param_1,1);
    DrawTextA(param_1,param_2,param_8,&local_10 /*rect+1*/,param_6);
    SetTextColor(param_1,param_4);
    DrawTextA(param_1,param_2,param_8,param_3,param_6);
    ```

  - So it is **white text with a 1 px black drop shadow down-right**.
  - Format 0x824 = `DT_SINGLELINE|DT_VCENTER|DT_NOPREFIX`, left-aligned at
    the box's x (no inset).
  - The background is transparent: a copy of the window background (`bg`
    surface `param_8`).
  - Selected text (with focus) is drawn opaque, with background `+0xe4`
    (white) and text `+0xe8` (black).
  - **Font:** game options never calls vtable `+0x18`. The field keeps the
    default from `FUN_005ff910`: `iVar3 = FUN_00603850(); if (iVar3 != 0)
    FUN_00600290(iVar3);`, the game root's `+0xb0`.
    hyp: That is the 14 px regular Arial default noted in
    `manufacturing-build-selection.md` (font-10 metrics). The value written
    to root `+0xb0` was not located.
- **Name source:** `FUN_00401860(header, k[, mp])` reads the slot header
  (`FUN_00415cd0` → `FUN_00411900`), which is:
  - a name string at `+4`;
  - six dwords `+0x10..+0x24`.

  The name text is `FUN_00604f90(name)`.
- **Faction icon:** drawn only when the name is non-empty:

  ```c
  if (iStack_44 == 2)      uVar12 = 0x2749;   // header +0x18 == 2  -> 10057
  else if (iStack_3c == 1) uVar12 = 0x2748;   // header +0x20 == 1  -> 10056
  else { if (iStack_3c != 2) goto LAB_0040753b;   uVar12 = 0x2747; }  // +0x20 == 2 -> 10055
  ...
  uVar12 = FUN_006037f0(8,uVar12); iVar9 = FUN_005fbd20(uVar12,uVar15);
  FUN_005fd0f0(iVar7 /* FUN_005ff440(): window +0x94 bg surface */,0x55,uVar13 /* y+2 */);
  ```

  - COMMON 10056: red Alliance crest. 10055: green Imperial crest. 10057: a
    picture, the multiplayer save. Each is 26x19, checked in
    `assets/references/ref-ui-full/common-dll/`.
  - `+0x18` is the game type: 1 = single-player, 2 = multiplayer. The
    single-player Load button is enabled only when it equals 1:
    `uVar10 = (uint)(iStack_1c == 1)`. `+0x20` is the side, 1 = Alliance
    and 2 = Empire. hyp: these names follow the bitmaps and the side
    colours (A0's red-crested row 2 is named "DS_flip_allian").
  - The icon is blitted with colour-key transparency (`FUN_005fd0f0` passes
    key byte `+8`) **into the window's background surface** at (85,y+2).
    It sits between the save button (34..76) and the edit box (119..).
- **Empty slot:** `sVar4 = FUN_005f3070()` (name length) is 0, so it calls
  `FUN_00604f90()`. `FUN_00408f10` shows the argument is `&DAT_006b120c`,
  a `.bss` empty string. **Nothing is drawn: no text and no icon.** The bare
  background shows through.
  hyp: The icon is burned into the background and `FUN_00408f10` never
  erases it. A slot that becomes empty keeps its old icon until the window
  is rebuilt.
- **Load button**:
  - (287,y,41,20), id `0x78+k`;
  - bitmaps 0x2741/0x2742 (10049/10050), disabled 0x2743 (10051);
  - tooltip 0x1011.

## 4. Sliders (`FUN_00604570`, window class "slider", vtable `0x66e010`)

The call is `FUN_00604570(module, x, y, parent, id, orientation=2
(horizontal), bgSurface, trackBmp, thumbBmp, value, max=100)`.
- Music: id 0x8d, track 0x2744 (10052), thumb 0x2746 (10054), tooltip 0x1012.
- Effects: id 0x8e, track 0x2745 (10053), thumb 10054, tooltip 0x1013.
- Track: 201x54. Thumb: 11x47.

Constructor:

```c
param_1[0x2a] = FUN_005fbd20(param_2,param_9,2);          // track
param_1[0x2b] = thumb; param_1[0x27] = thumb w; param_1[0x28] = thumb h;
param_1[0x2d] = FUN_005fbda0(iVar2,iVar3,param_8);         // composite, track-sized
param_1[0x2e..0x31] = 0,0,iVar2,iVar3;                     // hit rect (0,0,201,54)
param_1[0x32] = iVar2 / 2; ...
else if (param_1[0x29] == 2) {
  param_1[0x35] = 0;
  param_1[0x32] = (((uint)(iVar2 * 100) / (uint)param_1[0x38]) * param_1[0x37]) / 100;  // thumb centre x
  param_1[0x34] = iVar3 / 2 - (int)param_1[0x28] / 2;                                    // thumb y
}
```

- Thumb centre x = `(201*100/100) * value / 100` = `floor(201*value/100)`,
  in slider-local pixels.
- Thumb y = 54/2 - 47/2 = 27 - 23 = 4.

Paint (`FUN_00604a50`):

```c
FUN_005fcc30(composite,0,0, x(+0x94), y(+0x98), 0,0);   // copy the parent background under the slider
FUN_005fd0f0(composite,0,0);                            // track at (0,0), colour-keyed
...
iVar3 = *(int *)(param_1 + 200) - *(int *)(param_1 + 0x9c) / 2;  // thumb left = cx - 5
local_4 = ((*(int *)(param_1 + 200) - uVar1) * *(int *)(param_1 + 0xe0)) / local_4; // value = cx*100/clientW
*(uint *)(param_1 + 0xdc) = local_4;
FUN_005fd0f0(composite /*thumb*/, iVar3, iVar2);        // thumb at (cx-5, 4)
FUN_005fc140(hdc,0x40,0xcc0020,...)                     // SRCCOPY the composite to the screen
```

`uVar1` = min `+0xd4` = 0, so the divisor falls back to `+0x38`. That is the
client width, set by WM_SIZE (`FUN_00600310`: `piVar6[0xe] = param_4 &
0xffff`), = 201.

- **The whole track bitmap is drawn every time. No part of the track
  differs on either side of the thumb.** The thumb is simply drawn over it.
- In 8-bit mode the colours come from the game palette (`SelectPalette(hdc,
  DAT_006be5a0)`).
  A0 `tooltip/200-options.png` shows palette entry 254 (cyan in the
  DIBs) as white in all 1328 such pixels of COMMON 20002 and the 664 of
  each track.
- The track's bottom-left pixel is magenta, the key colour: A0 shows the
  window's background (black) there.

Interaction (`Class_0066e010_vt14` = `FUN_00604b80`, `FUN_00604950`):
- **Button down** (0x201): `FUN_00601080` (hyp: SetCapture). If the cursor
  is inside (0,0)-(201,54), it moves the thumb.
- **Move** (0x200) with the left button held and the capture: it moves the
  thumb, then `PostMessageA(parent, WM_COMMAND, id, value(+0xdc))`.
- **Button up** (0x202): it releases the capture and posts the same
  message.
- The cursor x is accepted only when
  `left + 5 < x < right - 5 && top < y < bottom`, that is x in 6..195.
  So a drag yields values `x*100/201` = 2..97. A drag never gives exactly 0
  or 100. Those come only from the initial value.
- The cursor point is written into `+0xc8` before the bounds check. A
  rejected point does not repaint, but it changes the next WM_PAINT.

Volume ↔ slider transfer:
- Initial value (`FUN_00407180` at `40773b`-`407780`, constants at
  `0x6585d8`.. = 0.01, 2000.0, 0.05, 10.0, 100.0):

  ```
  vol = music: DAT_006b14a8 (FUN_00401ac0); effects: root+300 (FUN_00401a80)   // DirectSound hundredths of dB
  if (vol < -9600 /*0xffffda80*/) value = 0;
  else value = (int) floor( pow(10, vol*0.01*0.05) * 100 );     // = floor(100 * 10^(vol/2000))
  ```

  `FUN_006197b0` is pow (fld 10.0, fild vol, fmul 0.01, fmul 0.05).
  `FUN_00619610` is floor: control word `0x173f` at `0x6acac0`, RC = down,
  then `frndint`.
- Command 0x8d/0x8e (`FUN_00406b80`, `406de5`-`406e0d`):

  ```
  value == 0  -> vol = -9600 (mov eax,0xffffda80)
  else        -> vol = (int) floor( log10(value*0.01) * 2000 )   // fldlg2; fild; fmul 0.01; fyl2x; fmul 2000.0
  ```

  The result goes to `FUN_00401aa0` (music; in tactical states also
  `FUN_00596b90`) or `FUN_00401a60` (effects).
- Round trip: a stored value v is shown at thumb x = floor(201·v/100). The
  first paint re-derives `+0xdc` = floor(x·100/201), so v=50 reads back as
  49. Clamp: v=100 puts the thumb centre at 201, and the thumb is clipped by
  the 201-wide composite.

## 5. Version label (+0x118)

`FUN_004084b0`, after `FUN_00601620(0,0x19,0x1b5,0x136,0x12,0x2000000,10,
0x25,3)`:

```c
local_34 = CONCAT22(DAT_0065d420,0x100c);  FUN_005f2fc0(&local_34);  // "Version: " (TEXTCOMM 4108, module 1)
FUN_005f35b0(s_1_01_00_006a7f44);          // "1.01.00", a literal in .data (bytes 31 2e 30 31 2e 30 30 00)
FUN_005f30d0(local_24);                    // append
uVar1 = FUN_00406840();                    // &DAT_006b06f8, the language name
FUN_005f35b0(uVar1);
FUN_005f35b0(&DAT_006a7f40);               // " " (bytes 20 00)
FUN_005f30d0(local_24); FUN_005f30d0(local_18);   // append " " then the language
uVar1 = FUN_00583c40(); FUN_00601aa0(uVar1);      // set the label text
```

- The result is "Version: 1.01.00" + " " + language.
- The version number is **hard-coded, not read from a resource**.
- The language comes from `FUN_00406850`:
  `GetFileVersionInfoA("textstra.dll")`, then
  `VerQueryValueA("\VarFileInfo\Translation")`, then
  `VerLanguageNameA(langId, &DAT_006b06f8, 0xff)`. For 0x0409 this is
  "English (United States)".
- Rect (25,437)-(335,455). Font 10 (14 px regular Arial). Black
  `0x2000000`. Format `DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOCLIP`. So it
  is centred at (180,446), which matches the capture.

## 6. Other

- All text objects are painted by `Class_00658600_vt38` (`FUN_00408bc0`,
  vtable slot `0x658638` → `0x408bc0`) in WM_PAINT after the background.
- The music button 0x99 uses bitmaps 0x2738/0x2739 (10040/10041), state 2
  0x273a and state 4 0x2739. It is set selected (`FUN_006030c0`) when flag
  0x100 is set.
- The switches use 0x273b/0x273c, state 2 0x273d and state 4 0x273c. They
  are disabled (vtable `+0x10`) in states 20-23.
