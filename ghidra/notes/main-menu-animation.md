# Main-menu animated buttons, difficulty buttons and galaxy lever (REBEXE.EXE)

Sources: `~/Games/rebexe-decompile/decompiled-typed/FUN_*.c`, plus `objdump -d`
of `LAB_004054a0` (not a Ghidra function) and `objdump -s` of the vtables, both
from `~/Games/Heroic/Star Wars - Rebellion/REBEXE.EXE`.

## 0. Summary

* All eight animated controls (Empire, Alliance, Load, Exit, Head-to-head and the
  three difficulty buttons) are driven by **one** periodic multimedia timer that
  the menu owns. The timer fires every **66 ms** (`0x42`), or about 15.2 fps. It
  runs **always**, whether or not the button is hovered or enabled. There is no
  hover handling at all.
* The frames form a **circular list**, so the animation **loops** forward
  (no ping-pong, no stop). A 15-frame button cycles in about 0.99 s and a
  30-frame button in about 1.98 s. All buttons tick in lockstep because one
  timer advances them all, in the same callback.
* The so-called "rest" art (`0x271d`, `0x271b`, `0x2719`, `0x2717`, `0x2715`,
  `0x2c09`/`0x2c0b`/`0x2c0a`) is the **pressed** bitmap (slot `+0x98`). It is
  shown only while the left button is held down on the control. It is **not**
  an idle image. For the difficulty buttons, the same id is also loaded into the
  "selected" slot (`+0xa0`).
* The selected difficulty shows its static `+0xa0` art (11273/11275/11274). The
  two unselected difficulty buttons animate continuously.
* The galaxy lever is a 3-position ping-pong lever. Small (the default,
  `menu[0x3b]=1`) is position 2, shown as **10003 (`0x2713`)**. Medium is
  position 1 (10002) and Large is position 0 (10001). 10015 and 10014 are the
  in-between frames drawn while the lever is pressed.
* Sounds are played on **mouse-down**, never on hover.

## 1. Animated button class (`FUN_00603870`, vtable `0x0066df88`)

The constructor `FUN_00603870(this, parent, x, y, w, h, owner, cmd, first_frame, rest_art, frame_count, ...)`
works as follows:

```c
FUN_00602150(param_2,param_3,param_4,param_5,param_6,param_7,param_8,0,param_10,param_12,param_13,param_14);
FUN_0060f2d0();                       // frame list object at this+0x114 (count +0x118, head +0x11c)
*param_1 = &PTR_Class_0066df88_vt0_0066df88;
FUN_00603aa0(param_9,param_11);       // load frames first..first+count-1
```

The base is `FUN_00602150`, which builds the `"CoolStrobeButton"` window, vtable
`0x0066df58`. Its signature is `(parent,x,y,w,h,owner,cmd,normal_art,pressed_art,...)`.
The animated subclass passes **normal = 0** and **pressed = rest_art**:

```c
param_1[9]    = param_8;              // +0x24 command id
param_1[0x31] = 0x10020;              // +0xc4 style/state flags
param_1[0x25] = FUN_005fbd20(param_2,param_9,10);   // +0x94 normal image (0 here)
param_1[0x26] = FUN_005fbd20(param_2,param_10,10);  // +0x98 pressed image  <- "rest" art
if (param_13 == 0) param_1[0x31] |= 0x10;           // pixel-mask hit test
```

Other image slots are set by `FUN_00603150(this, state, res)`:
state 0 → `+0x94`, 1 → `+0x98`, 2/0x40 → `+0x9c` (disabled), 4 → `+0xa0` (selected).

The frames are loaded by `FUN_00603aa0`:

```c
do { iVar4 = FUN_005fbd20(uVar1,uVar5 + param_2,2);  ... FUN_0060f320(FUN_0060f3c0(iVar4)); uVar5++; }
while ((uVar5 & 0xffff) < param_3);
*(param_1 + 0x120) = *(param_1 + 0x11c);   // current-frame node = head (first frame)
```

`FUN_0060f320` appends to a **circular** doubly-linked list. Each node holds
`+4` = bitmap, `+8` = next and `+0xc` = prev. When the list is empty, the code
sets `node.next = node`. Otherwise it sets `tail.next = node` and
`node.next = head`. Following `next` therefore wraps from the last frame to the
first.

### Frame advance, `FUN_00603a50`

```c
iVar1 = FUN_00603120(0x40);          // (flags +0xc4 & 0x40) != 0 ?
if (iVar1 == 0) {
  if (*(int *)(param_1 + 0x120) != 0) {
    iVar1 = *(int *)(*(int *)(param_1 + 0x120) + 8);   // node = node->next (wraps)
    *(int *)(param_1 + 0x120) = iVar1;
    *(undefined4 *)(param_1 + 0x94) = *(undefined4 *)(iVar1 + 4);  // normal image = frame
    InvalidateRect(*(HWND *)(param_1 + 0x18),(RECT *)0x0,0);
    return;
  }
  *(undefined4 *)(param_1 + 0x94) = 0;
}
```

The only gate is style flag `0x40`. The menu never sets `0x40` on these
buttons; it uses only `8` and `4` (see section 2). Hover, focus, enable and the
selected flag `4` do not stop the animation.

### The timer

At the end of `FUN_00405560`:

```c
iVar3 = FUN_00603b90(&LAB_004054a0);     // stored in menu+0x9c
if (iVar3 != 0) FUN_00602060(0,0);      // (re)start it
```

`FUN_00603b90` calls `FUN_00601f40(0x42,1,10)`, and the callback pointer sits
at `+0x38` (`Class_0066dfb8_vt4` calls it). `FUN_00601f40(delay, fuEvent, resolution)`:

```c
MVar1 = timeSetEvent(param_2 /*0x42=66 ms*/, param_4 /*10, clamped to wPeriodMin*/,
                     FUN_006020d0, (DWORD_PTR)param_1, param_3 /*1 = TIME_PERIODIC*/);
```

It is therefore a periodic `timeSetEvent` with a 66 ms period. It is not
`SetTimer` or `WM_TIMER`. `FUN_006020d0` takes a critical section and calls
`vt+4`, which calls `LAB_004054a0`. Disassembly of that label:

```
4054a0: mov eax,[0x6b06f4]        ; main-menu window (set in FUN_00404c30)
        test eax,eax ; je ret
        mov ecx,[eax+0x104] ; test ecx,ecx ; jne ret   ; frozen while menu[0x41] != 0
        mov ecx,[eax+0xb0] ; call 0x603a50   ; Empire
        mov ecx,[eax+0xb4] ; call 0x603a50   ; Alliance
        mov ecx,[eax+0xc8] ; call 0x603a50   ; Load
        mov ecx,[eax+0xb8] ; call 0x603a50   ; Easy
        mov ecx,[eax+0xbc] ; call 0x603a50   ; Intermediate
        mov ecx,[eax+0xc0] ; call 0x603a50   ; Expert
        mov ecx,[eax+0xc4] ; call 0x603a50   ; Head-to-head
        mov eax,[eax+0xcc] ; jmp  0x603a50   ; Exit
```

(Each call is guarded by a null check.) `menu+0x104` (`param_1[0x41]`) is set
to 1 only by the Credits command 0x73 in `FUN_00406000`
(`param_1[0x40] = 1; param_1[0x41] = 1;`), so the buttons freeze while the
credits run. Otherwise they animate continuously.

### What is drawn: paint `FUN_00602d30` (vtable `0x66df58` / `0x66df88` slot `+0x24`)

The paint state is `(flags & 1)`, where flag 1 is "pressed". `WM_LBUTTONDOWN`
sets it with `FUN_006030c0(1)`.

```c
if (flags & 2) { +0xf8 = 0; state = 2; }          // disabled
case 1 /*pressed*/: if (+0xf8 /*hit at mouse-down*/) img = +0x98 /*pressed = "rest" art*/;
                    else img = +0x94;                // current animation frame
case 2/0x40:        img = +0x9c ? +0x9c : +0x94;
case 0 (normal):    if (flags & 4) img = +0xa0 ? +0xa0 : +0x94;      // selected
                    else if (flags & 0x40) img = +0x9c ? +0x9c : +0x94;
                    else img = +0x94;                // current animation frame
if (img != 0) blit ...                               // img == 0 -> nothing drawn
```

* **Idle and unhovered:** the control shows the current animation frame.
* **Pressed with the pointer inside:** it shows the "rest" art (10013, 10011,
  10009, 10007, 10005, 11273/11275/11274).
* **Before the first tick:** the animated subclass's `WM_CREATE` handler
  (`FUN_006039a0`, msg 1) sets `+0x94 = node(+0x120)->bitmap`. That runs inside
  window creation, before `FUN_00603aa0` loads the frames, so `+0x94` is still 0.
  Nothing is drawn (the cockpit shows through) until the first 66 ms tick.
  The first tick moves to `head->next`, so the displayed sequence is
  `first+1, ..., first+count-1, first, first+1, ...`. In practice the control
  simply loops through all `count` frames.

`+0xf8` is the hit flag. `FUN_006035f0` sets it on mouse-down and mouse-up only,
using a pixel-mask hit test against the current image (flag `0x10`).

## 2. Difficulty buttons (0x6e / 0x6f / 0x70)

They are created with `FUN_00603870(..., 0x6e, 0x2b35, 0x2c09, 0x1e, ...)` and the
same pattern for the others, so their pressed art (`+0x98`) is 0x2c09/0x2c0b/0x2c0a.
After creation, in `ghidra/notes/FUN_00405560.c`:

```c
FUN_006030c0(*(void **)(param_1 + 0xb8),8);          // flag 8 = toggle button
FUN_00603150(*(void **)(param_1 + 0xb8),4,0x2c09);   // selected art (+0xa0) = 11273
...
if (*(int *)(param_1 + 0xf8) == 0) FUN_006030c0(*(void **)(param_1 + 0xb8),4); // flag 4 = selected
```

The same applies to 0xbc (0x2c0b, selected when `menu+0xf8 == 1`) and 0xc0
(0x2c0a, when `== 2`).

On `WM_LBUTTONUP` inside, `FUN_006028c0` toggles the selection:
`if (flags & 8) flags ^= 4;`. `FUN_00406000` cases 0x6e/0x6f/0x70 then
re-select the clicked button (so it cannot be deselected), clear flag 4 on the
other two, and set `menu[0x3e]` (difficulty) to 0, 1 or 2. Ghidra lost the
`this` pointers there, so this reading is inferred from the call pattern.

Rendering follows from that:
* The **selected** difficulty draws `+0xa0`, a static 11273 (Easy), 11275
  (Intermediate) or 11274 (Expert).
* The **unselected** difficulties draw `+0x94`, the animation frame. The timer
  ticks `+0xb8/+0xbc/+0xc0` too, and `FUN_00603a50` checks only `0x40`, so they
  **animate continuously** (30 frames: 11061-11090, 11091-11120, 11121-11150).
* The selected button's list also keeps advancing in the background. When it is
  deselected, it resumes at whatever frame the shared clock has reached.
* A pressed difficulty button shows `+0x98`, which is the same bitmap as its
  selected art.

## 3. Galaxy lever, `FUN_00603cd0` (vtable `0x0066dfc8`, wndproc `FUN_00604000`, paint `FUN_00604170`)

Call: `FUN_00603cd0(parent, 0xf2,0x10f,0x2c,0x2f, menu, 0x6a, 0x2713,0x271f,0x2712,0x271e,0x2711, ...)`.

```c
FUN_00602150(..., param_8 /*0x6a*/, param_11 /*0x2712 normal*/, param_13 /*0x2711 pressed*/, ...);
param_1[0x45] = bmp(param_9)  /* +0x114 = 0x2713 = 10003 */;
param_1[0x46] = bmp(param_10) /* +0x118 = 0x271f = 10015 */;
param_1[0x47] = bmp(param_11) /* +0x11c = 0x2712 = 10002 */;
param_1[0x48] = bmp(param_12) /* +0x120 = 0x271e = 10014 */;
param_1[0x49] = bmp(param_13) /* +0x124 = 0x2711 = 10001 */;
param_1[0x4d] = 2;   // +0x134 position
param_1[0x4e] = 1;   // +0x138 direction (1 = decreasing)
```

Resting paint (`FUN_00604170`, case 0):

```c
uVar2 = *(uint *)(param_1 + 0x134) & 3;
if (uVar2 == 0) iVar3 = +0x124;   // 10001
else if (uVar2 == 1) iVar3 = +0x11c;  // 10002
else if (uVar2 == 2) iVar3 = +0x114;  // 10003
```

The command handler `FUN_00406000` case 0x6a converts the lever position into
the menu's galaxy size:

```c
uVar1 = *(uint *)(param_1[0x2b] + 0x134) & 3;   // lever = menu+0xac
if (uVar1 == 0) param_1[0x3b] = 3; else if (uVar1 == 1) param_1[0x3b] = 2;
else if (uVar1 == 2) param_1[0x3b] = 1; else param_1[0x3b] = 0;
```

`FUN_00404c30` initialises `param_1[0x3b] = 1`. `FUN_00405050` paints the
selected screen at x 0x122 for size 1, 0x146 for 2 and 0x16a for 3.

| Galaxy (`menu[0x3b]`) | Original label | Rust enum | Lever position | Resting lever art |
|---|---|---|---|---|
| 1 (default) | Small | Standard | 2 | **10003 (`0x2713`)** |
| 2 | Medium | Large | 1 | 10002 (`0x2712`) |
| 3 | Large | Huge | 0 | 10001 (`0x2711`) |

The pressed paint (case 1, while the pointer is inside) steps the position in
ping-pong order and draws a transition frame. In-between art: 10015 between
positions 2 and 1, and 10014 between 1 and 0.

```c
if (dir == 0) { pos++; pos==1 -> 10014 (+0x120); pos==2 -> 10015 (+0x118);
                else(3) -> 10015, dir=1, pos=1; }
else          { pos--; pos==0 -> 10014; pos==1 -> 10015;
                else(-1) -> 10014, dir=0, pos=1; }
FUN_00610c30(this+0xe0);   // plays the lever sound (8000)
```

Starting from the default (pos 2, dir 1), clicks give
Small → Medium → Large → Medium → Small → ...:

| Click | Lever move | Frame shown while held | Resting art after release |
|---|---|---|---|
| 1 | 2 → 1 | 10015 | 10002 |
| 2 | 1 → 0 | 10014 | 10001 |
| 3 | 0 → 1 (direction flips) | 10014 | 10002 |
| 4 | 1 → 2 | 10015 | 10003 |

During `WM_LBUTTONDOWN`/`UP`, `FUN_00604000` temporarily swaps `+0x94` for the
current position's art so that the pixel-mask hit test uses the visible lever
shape. The lever is not ticked by the timer and does not animate.

## 4. Hover and sound

* The `CoolStrobeButton` wndproc (`FUN_006028c0`) handles only messages
  `0xf, 1, 3, 0x10, 0x14, 0x111, 0x201, 0x202, 0x203` and `0x311`. There is
  **no `WM_MOUSEMOVE` (0x200)** and no hover state. Animation speed never
  changes, and nothing happens on hover. The animated override `FUN_006039a0`
  adds only msg 1 and `WM_PAINT`.
* `FUN_00602840(this, id)` stores the click sound:
  `*(ushort*)(this+0xdc) = id; *(this+0xe4) = id; flags(+0xf0) |= 1`.
  It is played on **`WM_LBUTTONDOWN`** (and 0x203 double-click):
  `if ((short)param_1[0x37] /*+0xdc*/ != 0) FUN_00610c30(param_1 + 0x38 /*+0xe0*/);`,
  which leads to `FUN_006115f0`, the sound player.
  Ids: 0x1f44 (8004) for Credits, Alliance, Empire, Load, Game type and the
  difficulty buttons; 8000 for the lever, which plays it from its pressed
  paint; 0x1f41 (8001) for Head-to-head; 0x1f42 (8002) for Exit.
* The `WM_COMMAND` (0x111 to the owner) is posted on `WM_LBUTTONUP` when the
  pointer is still inside. The animated button also posts `0x40e` on mouse-down.

## Port implications

1. Draw every animated control's current frame at all times, advancing one
   frame per 66 ms on a shared clock (all controls in lockstep) and looping.
   Do not tie the animation to hover.
2. Draw the "rest" art only while the control is pressed with the pointer
   inside. For difficulty buttons, also draw it for the selected one.
3. Freeze the animation while Credits is open (`menu+0x104`).
4. The lever rests at 10003 for Small/Standard, 10002 for Medium/Large and
   10001 for Large/Huge. Use 10015/10014 as the press frames, with the
   ping-pong order shown above.
5. Play the click sound on mouse-down. Play nothing on hover.
