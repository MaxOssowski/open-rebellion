#!/usr/bin/env python3
"""Find which TACTICAL destroyed frame (3360..3375) an A0 fireball shows.

usage: python3 -I tools/interface-parity/fireball-frame-a0.py A0.png RUNTIME_DIR

A0.png is a 640 x 480 capture (TAC-03-C004). RUNTIME_DIR is the decoded
TACTICAL3D runtime folder (data/base/ui/tactical-dll/TACTICAL3D/runtime) with
its manifest.json. Both are local and untracked; nothing is written.

Each frame is decoded through the battle palette 5531, its first index is
transparent (the port's rule, tactical_assets.rs), and it is drawn as a
square decal with nearest sampling. The decal size is first estimated from
the frame's opaque box against A0's fire box, then searched within a few
pixels of size and position. A pixel counts as fire when its colour is one
of the frames' palette colours. Per frame the best fit is printed as JSON:
pixels with the same colour, fire of another colour, fire only in A0, and
fire only in the frame.
"""
import json
import os
import struct
import sys

from PIL import Image

WINDOW = (130, 260, 240, 350)


def load_runtime(runtime):
    manifest = json.load(open(os.path.join(runtime, "manifest.json")))
    by_id = {e["id"]: e for e in manifest["textures"] if e.get("identifier_kind") == "id"}

    def payload(resource):
        return open(os.path.join(runtime, by_id[resource]["object"]), "rb").read()

    palette_bytes = payload(5531)
    assert palette_bytes[:8] == b"ORTPAL00"
    rgb = palette_bytes[16:16 + 768]
    palette = [tuple(rgb[k * 3:k * 3 + 3]) for k in range(256)]
    frames = {}
    for resource in range(3360, 3376):
        data = payload(resource)
        assert data[:8] == b"ORTINDEX"
        _, width, height, _, _, pixels, _ = struct.unpack_from("<7I", data, 8)
        indices = data[36:36 + pixels]
        transparent = indices[0]
        frames[resource] = (width, height,
                            [None if v == transparent else palette[v] for v in indices])
    return frames


def score(frame, a0fire, size, cx, cy, confusion=None):
    width, height, pixels = frame
    x0, y0, x1, y1 = WINDOW
    left, top = cx - size / 2.0, cy - size / 2.0
    same = other = a0_only = frame_only = 0
    for y in range(y0, y1):
        v = (y + 0.5 - top) / size
        row = int(v * height) if 0 <= v < 1 else None
        for x in range(x0, x1):
            u = (x + 0.5 - left) / size
            colour = None
            if row is not None and 0 <= u < 1:
                colour = pixels[row * width + int(u * width)]
            seen = a0fire.get((x, y))
            if colour is None and seen is None:
                continue
            if colour is None:
                a0_only += 1
            elif seen is None:
                frame_only += 1
            elif colour == seen:
                same += 1
            else:
                other += 1
                if confusion is not None:
                    key = f"{colour}->{seen}"
                    confusion[key] = confusion.get(key, 0) + 1
    return same, other, a0_only, frame_only


def main():
    a0_path, runtime = sys.argv[1], sys.argv[2]
    frames = load_runtime(runtime)
    fire = {c for _, _, px in frames.values() for c in px if c is not None}
    a0 = Image.open(a0_path).convert("RGB")
    x0, y0, x1, y1 = WINDOW
    a0fire = {}
    for y in range(y0, y1):
        for x in range(x0, x1):
            colour = a0.getpixel((x, y))
            if colour in fire:
                a0fire[(x, y)] = colour
    xs = [x for x, _ in a0fire]
    ys = [y for _, y in a0fire]
    box = (min(xs), min(ys), max(xs), max(ys))
    print(json.dumps({"a0_fire_pixels": len(a0fire), "a0_box": box,
                      "a0_colours": sorted(set(a0fire.values()))}))
    box_w, box_h = box[2] - box[0] + 1, box[3] - box[1] + 1
    box_cx, box_cy = (box[0] + box[2] + 1) / 2.0, (box[1] + box[3] + 1) / 2.0
    for resource, frame in frames.items():
        width, height, pixels = frame
        opaque = [(k % width, k // width) for k, c in enumerate(pixels) if c is not None]
        bx0 = min(x for x, _ in opaque)
        bx1 = max(x for x, _ in opaque) + 1
        by0 = min(y for _, y in opaque)
        by1 = max(y for _, y in opaque) + 1
        from_w = box_w * width / (bx1 - bx0)
        from_h = box_h * height / (by1 - by0)
        best = None
        for size in range(max(8, int(min(from_w, from_h)) - 3), int(max(from_w, from_h)) + 4):
            ocx = box_cx - ((bx0 + bx1) / 2.0 / width - 0.5) * size
            ocy = box_cy - ((by0 + by1) / 2.0 / height - 0.5) * size
            for dx in range(-3, 4):
                for dy in range(-3, 4):
                    counts = score(frame, a0fire, size, ocx + dx, ocy + dy)
                    fit = counts[0] - counts[1] - counts[2] - counts[3]
                    if best is None or fit > best[0]:
                        best = (fit, size, ocx + dx, ocy + dy, counts)
        _, size, cx, cy, (same, other, a0_only, frame_only) = best
        confusion = {}
        score(frame, a0fire, size, cx, cy, confusion)
        print(json.dumps({
            "resource": resource, "opaque_box": [bx0, by0, bx1, by1],
            "size_from_width": round(from_w, 1), "size_from_height": round(from_h, 1),
            "decal_size": size, "centre": [round(cx, 2), round(cy, 2)],
            "same_colour": same, "other_colour": other,
            "a0_only": a0_only, "frame_only": frame_only,
            "top_other_colour_pairs": sorted(confusion.items(), key=lambda kv: -kv[1])[:4],
        }), flush=True)


if __name__ == "__main__":
    main()
