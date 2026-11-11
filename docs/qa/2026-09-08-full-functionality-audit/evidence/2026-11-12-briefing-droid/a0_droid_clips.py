"""Match each A0 briefing frame's droid box to a clip frame, then date the voices.

usage: python3 a0_droid_clips.py alliance|empire [repo root]

It decodes every voiced clip of the briefing DLL as the game does
(FUN_00404440: the definition record's first frame is a BMP, the next
count - 1 are type-302 deltas added to it, FUN_0041c7a0), draws each frame
unscaled at its aperture's corner (FUN_0042adb0: Alliance 541, 337;
Empire 0, 347), and looks for the frame whose opaque pixels all equal the
A0 box. Then it dates each clip's start, so its voice's (FUN_00403f70):
frame k of a series taken every P seconds showing frame f started the clip
f ticks before k * P. Needs the staged data in data/base/ui; reads no game
data into the repository.
"""
import json
import os
import struct
import sys

from PIL import Image

side = sys.argv[1]
root = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(__file__), '../../../../..')
UI = os.path.join(root, 'data/base/ui')
A0 = os.path.join(root, '.artifacts/a0-vm')
if side == 'empire':
    dll, folder = 'embrief-dll', 'briefing-empire'
    defs = list(range(0x1033, 0x1041)) + list(range(0x107c, 0x1083))
    box, corner = (28, 352, 110, 480), (0, 347)
    series = [(lambda n: True, 2.0)]
else:
    dll, folder = 'albrief-dll', 'briefing-alliance'
    defs = list(range(0x103b, 0x1049)) + list(range(0x107c, 0x1083))
    box, corner = (536, 344, 606, 452), (541, 337)
    series = [(lambda n: n < 'b100', 1.5), (lambda n: n >= 'b100', 3.0)]


def bmp(i):
    b = open(os.path.join(UI, dll, 'BMP/%d.bmp' % i), 'rb').read()
    off = struct.unpack('<I', b[10:14])[0]
    hs = struct.unpack('<I', b[14:18])[0]
    w, h = struct.unpack('<ii', b[18:26])
    count = struct.unpack('<I', b[46:50])[0] or 256
    pal = [(b[14 + hs + 4 * k + 2], b[14 + hs + 4 * k + 1], b[14 + hs + 4 * k]) for k in range(count)]
    stride = (w + 3) & ~3
    rows = abs(h)
    idx = []
    for r in range(rows):
        src = rows - 1 - r if h > 0 else r
        idx += list(b[off + src * stride:off + src * stride + w])
    return w, rows, idx, pal


def add_delta(i, w, h, idx):
    path = os.path.join(UI, dll, 'TYPE302/%d.bin' % i)
    if not os.path.exists(path):
        return
    b = open(path, 'rb').read()
    payload = b[17 + 4 * h:]
    for r in range(h):
        c = struct.unpack('<I', b[17 + 4 * r:21 + 4 * r])[0]
        x, literal = 0, False
        while x < w:
            n = payload[c]
            c += 1
            if literal:
                for k in range(n):
                    idx[r * w + x + k] = (idx[r * w + x + k] + payload[c + k]) & 255
                c += n
            x += n
            literal = not literal


def wave_of(d):
    return struct.unpack('<5H', open(os.path.join(UI, dll, 'RCDATA/%d.bin' % d), 'rb').read()[:10])


W, H = box[2] - box[0], box[3] - box[1]
ox, oy = box[0] - corner[0], box[1] - corner[1]
candidates = []
for d in defs:
    if not os.path.exists(os.path.join(UI, dll, 'RCDATA/%d.bin' % d)):
        continue
    _, wave, _, count, first = wave_of(d)
    w, h, idx, pal = bmp(first)
    for f in range(count):
        if f:
            add_delta(first + f, w, h, idx)
        points = [(y * W + x, pal[idx[(y + oy) * w + x + ox]])
                  for y in range(H) if 0 <= y + oy < h
                  for x in range(W) if 0 <= x + ox < w and idx[(y + oy) * w + x + ox]]
        candidates.append((wave, f, count, points))

names = sorted(n for n in os.listdir(os.path.join(A0, folder)) if n.endswith('.png'))
matches = {}
for name in names:
    image = Image.open(os.path.join(A0, folder, name)).convert('RGB').crop(box)
    pixels = list(image.getdata())
    matches[name] = [(wave, f, count) for wave, f, count, points in candidates
                     if points and all(pixels[i] == c for i, c in points)]
    print(name, matches[name][:3])

for keep, period in series:
    chosen = [n for n in names if keep(n)]
    seen = []
    for k, name in enumerate(chosen):
        # A clip's first and last frames are left out: the first is also
        # the idle still, the last holds while its voice plays on.
        hits = [m for m in matches[name] if 0 < m[1] < m[2] - 1]
        if len({m[0] for m in hits}) == 1:
            seen.append((k * period, hits[0][0], hits[0][1]))
    pairs = [(a, b) for a, b in zip(seen, seen[1:]) if a[1] == b[1]]
    tick = sum(b[0] - a[0] for a, b in pairs) / sum(b[2] - a[2] for a, b in pairs)
    starts = {}
    for t, wave, f in seen:
        starts.setdefault(wave, []).append(t - f * tick)
    print(json.dumps({'period': period, 'tick': round(tick, 4),
                      'starts': [[w, round(sum(v) / len(v), 2), round(max(v) - min(v), 2)]
                                 for w, v in starts.items()]}))
