// Lists the exact keyed STRATEGY GID marks (10130..10181) at every system's
// corner in 640x480 A0 captures, for both cockpits' origins.
//
// FUN_00425d00 puts a system's 17x17 cell at
//   x = (SYSTEMSD x * 607 >> 10) + 21 | 84, y = (y * 437 >> 10) + 25 | 27
// (side 1, the Alliance | otherwise), and FUN_0042cde0 blits the mark keyed
// on its first DIB pixel from that corner.
//
// Run from the repository root:
//   node docs/qa/2026-09-08-full-functionality-audit/evidence/2026-11-11-gid-display-states/findmarks-corners.mjs \
//     .artifacts/a0-vm/gid/d-popular-support.png ...
// Prints one JSON line per capture with hits: [mark, cockpit, system, id, x, y].
// A small mark (10149, 10150, 10154: 5 pixels) also matches inside a larger
// one of the same colour; take the largest mark found at a corner.
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";

const root = process.cwd();
const { PNG } = createRequire(path.join(root, "tools/interface-parity/"))("pngjs");
const bmpDir = path.join(root, "data/base/ui/strategy-dll/BMP");
const names = JSON.parse(fs.readFileSync(path.join(root, "data/base/textstra.json"), "utf8"));

function bmp(bytes) {
  const offset = bytes.readUInt32LE(10);
  const palette = 14 + bytes.readUInt32LE(14);
  const width = bytes.readInt32LE(18);
  const height = bytes.readInt32LE(22);
  const stride = (width + 3) & ~3;
  return {
    width, height,
    pixel(x, y) {
      const entry = palette + bytes[offset + (height - 1 - y) * stride + x] * 4;
      return [bytes[entry + 2], bytes[entry + 1], bytes[entry]];
    },
  };
}

// SYSTEMSD.DAT: 16-byte header, then 44-byte records; x and y at +36, +38.
const systemsd = fs.readFileSync(path.join(root, "data/base/SYSTEMSD.DAT"));
const systems = [];
for (let i = 0; i < systemsd.readUInt32LE(4); i += 1) {
  const at = 16 + i * 44;
  systems.push({ id: systemsd.readUInt32LE(at) & 0xffffff, text: systemsd.readUInt16LE(at + 20),
    x: systemsd.readUInt16LE(at + 36), y: systemsd.readUInt16LE(at + 38) });
}

const marks = {};
for (let id = 10130; id <= 10181; id += 1) {
  const file = path.join(bmpDir, `${id}.bmp`);
  if (!fs.existsSync(file)) continue;
  const b = bmp(fs.readFileSync(file));
  const key = b.pixel(0, b.height - 1).join();
  const pixels = [];
  for (let y = 0; y < b.height; y += 1) {
    for (let x = 0; x < b.width; x += 1) {
      const p = b.pixel(x, y);
      if (p.join() !== key) pixels.push([x, y, p]);
    }
  }
  if (pixels.length >= 3) marks[id] = pixels;
}

const corners = systems.flatMap((s) => [
  ["alliance", s, ((s.x * 607) >> 10) + 21, ((s.y * 437) >> 10) + 25],
  ["empire", s, ((s.x * 607) >> 10) + 84, ((s.y * 437) >> 10) + 27],
]);

for (const file of process.argv.slice(2)) {
  let png;
  try { png = PNG.sync.read(fs.readFileSync(file)); } catch { continue; }
  if (png.width !== 640 || png.height !== 480) continue;
  const hits = [];
  for (const [cockpit, s, cx, cy] of corners) {
    for (const [id, pixels] of Object.entries(marks)) {
      const exact = pixels.every(([x, y, p]) => {
        const i = ((cy + y) * 640 + cx + x) * 4;
        return png.data[i] === p[0] && png.data[i + 1] === p[1] && png.data[i + 2] === p[2];
      });
      if (exact) hits.push([Number(id), cockpit, names[s.text], s.id, cx, cy]);
    }
  }
  if (hits.length) console.log(JSON.stringify({ file, hits }));
}
