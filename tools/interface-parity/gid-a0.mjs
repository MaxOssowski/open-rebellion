// The GID against A0 2026-11-13-gid (gid_legend.rs, lib.rs's GID marks).
//
// Opens the GidA0 fixture (code 113, the Alliance): Kubindi explored,
// uninhabited and held by no side, Adar Tallon alone at Averam on a
// mission, on Popular Support. Then, against the A0 captures of the
// Alliance's game (days 243 to 258):
//
// - CMD-02-C020: Kubindi's no-side mark on Popular Support (B2), every
//   non-key pixel; its Calaron neighbours keep the unexplored mark.
// - CMD-02-C023: Averam's mark on Active Personnel (Alt+6) with Tallon on a
//   mission, the Alliance's largest (C2), every non-key pixel; every
//   unexplored system shows the no-side "None" mark there (A6).
// - CMD-02-C017: the legend button (command 0x134, FUN_00427270 through
//   FUN_00452ab0 with flags 3) answers only a double click
//   (Class_00659e38_vt14 swallows the press and the release and posts the
//   command on WM_LBUTTONDBLCLK; A1, A2: a press held or a click opens
//   nothing; A3: a double click opens the legend). The legend window for
//   Popular Support (A3), Idle Fleets (A5), Active Personnel (A7) and
//   Uprisings (A9) is compared with A0: its frame, close box and layout
//   bitmaps every pixel; each text's ink box within the accepted text-width
//   deviation; and the shaded galaxy under it (FUN_00452240, FUN_005fe050)
//   pixel for pixel outside the systems' marks, which follow each game's
//   state. An Alt+number display change closes the legend (A4).
// - CMD-02-C024: the both-sides mark 10160 (0x27b0, FUN_0042b330 case 0x21)
//   on Idle Fleets. The FleetsBothSides fixture (code 144, the Alliance)
//   puts one fleet of each side in orbit at its primary system. Against
//   A0 2026-11-18-edited-saves-7 a-02 (an unedited Alliance game, day 9,
//   the frame in which Fleet 2 reaches Coruscant, where the Imperial Fleet
//   3 is seen): Coruscant's cell (226, 150) shows 10160, and a-01 (day 5,
//   Fleet 2 en route) shows the Imperial 10150 there. The port's mark sits
//   at its system's FUN_00425d00 corner, every non-key pixel as A0's.
//
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node gid-a0.mjs [--out=DIR]
// Env:         CHROMIUM_PATH (default /usr/bin/chromium)
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
import { PNG } from "pngjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const SITE = path.join(ROOT, ".artifacts/interface-parity/site");
const A0 = path.join(ROOT, ".artifacts/a0-vm/2026-11-13-gid");
const A0_BOTH_SIDES = path.join(ROOT, ".artifacts/a0-vm/2026-11-18-edited-saves-7");
// sha256 first 12 hex, as that folder's README lists them.
const A0_BOTH_SIDES_FILES = {
  before: ["a-01-alhqd2-unedited-idle-fleets-fleet2-en-route-coruscant-10150-d5.png", "29e1f70b3f03"],
  both: ["a-02-alhqd2-unedited-idle-fleets-coruscant-10160-both-sides-d9.png", "b81e3db3c664"],
};
const args = Object.fromEntries(process.argv.slice(2).map((arg) => {
  const [key, value = "true"] = arg.replace(/^--/, "").split("=");
  return [key, value];
}));
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `gid-a0-${new Date().toISOString().replace(/[:.]/g, "-")}`));
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".orpk": "application/octet-stream", ".webm": "video/webm" };
// Fixture codes are the Scenario index plus one: 112 GidA0.
const GID_A0 = 113;
// 143 FleetsBothSides.
const FLEETS_BOTH_SIDES = 144;
const ALLIANCE = 1;
// FUN_00427270: the legend button, 30 by 30 at (55, 50) for the Alliance.
const BUTTON = [70, 65];
// FUN_00421c70: the legend's kept position starts at (150, 120).
const LEGEND = [150, 120];
// FUN_004522f0's layouts: Popular Support (the Alliance's), Idle Fleets,
// Active Personnel, Uprisings.
const LEGENDS = [
  { a0: "A3-legend-doubleclick-popular-support.png", layout: 0x27c7, alt: null },
  { a0: "A5-legend-doubleclick-idle-fleets.png", layout: 0x2d51, alt: "3" },
  { a0: "A7-legend-doubleclick-active-personnel.png", layout: 0x2d54, alt: "6" },
  { a0: "A9-legend-doubleclick-uprisings.png", layout: 0x2d50, alt: "2" },
];

function serve(root) {
  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url, "http://localhost");
    const file = path.join(root, decodeURIComponent(url.pathname === "/" ? "/index.html" : url.pathname));
    try {
      const bytes = await fs.readFile(file);
      response.writeHead(200, { "content-type": MIME[path.extname(file)] ?? "application/octet-stream" });
      response.end(bytes);
    } catch {
      response.writeHead(404);
      response.end();
    }
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}

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
const bitmaps = new Map();
async function strategy(id) {
  if (!bitmaps.has(id)) {
    bitmaps.set(id, bmp(await fs.readFile(path.join(ROOT, `data/base/ui/strategy-dll/BMP/${id}.bmp`))));
  }
  return bitmaps.get(id);
}
// A bitmap's pixels, without its key (the first DIB pixel, bottom left)
// when keyed.
async function artPixels(id, keyed = true) {
  const b = await strategy(id);
  const key = b.pixel(0, b.height - 1).join();
  const pixels = [];
  for (let y = 0; y < b.height; y += 1) {
    for (let x = 0; x < b.width; x += 1) {
      const p = b.pixel(x, y);
      if (!keyed || p.join() !== key) pixels.push([x, y, p]);
    }
  }
  return pixels;
}
const at = (png, x, y) => {
  const i = (y * png.width + x) * 4;
  return [png.data[i], png.data[i + 1], png.data[i + 2]];
};
const same = (a, b, x, y) => at(a, x, y).join() === at(b, x, y).join();

// SYSTEMSD.DAT: 16-byte header, then 44-byte records; x and y at +36, +38.
// FUN_00425d00 puts the Alliance's 17 by 17 cell at
// ((x * 607) >> 10) + 21, ((y * 437) >> 10) + 25.
const names = JSON.parse(await fs.readFile(path.join(ROOT, "data/base/textstra.json"), "utf8"));
const systemsd = await fs.readFile(path.join(ROOT, "data/base/SYSTEMSD.DAT"));
const CORNERS = new Map();
for (let i = 0; i < systemsd.readUInt32LE(4); i += 1) {
  const record = 16 + i * 44;
  CORNERS.set(names[systemsd.readUInt16LE(record + 20)],
    [((systemsd.readUInt16LE(record + 36) * 607) >> 10) + 21, ((systemsd.readUInt16LE(record + 38) * 437) >> 10) + 25]);
}
// The largest STRATEGY mark (10146..10160) whose every non-key pixel shows
// at `corner`.
async function markAt(png, [left, top]) {
  let found = null;
  for (let id = 10146; id <= 10160; id += 1) {
    if (id === 10159) continue;
    const pixels = await artPixels(id);
    if (pixels.every(([x, y, p]) => at(png, left + x, top + y).join() === p.join())
      && (!found || pixels.length > found.pixels)) found = { id, pixels: pixels.length };
  }
  return found?.id ?? null;
}
// The mark's non-key pixels that differ between the port and A0 at the
// same corner.
async function markDiff(port, a0, id, [left, top]) {
  const pixels = await artPixels(id);
  return { mark: id, pixels: pixels.length,
    different: pixels.filter(([x, y]) => !same(port, a0, left + x, top + y)).length };
}

// A mark's non-key pixels at `portCorner` against A0's at `a0Corner`, and
// a hash of each side's pixels (RGB, in the bitmap's row order).
async function markDiffAt(port, portCorner, a0, a0Corner, id) {
  const pixels = await artPixels(id);
  const hash = (png, [left, top]) => createHash("sha256")
    .update(Buffer.from(pixels.flatMap(([x, y]) => at(png, left + x, top + y)))).digest("hex");
  const different = pixels.filter(([x, y]) => at(port, portCorner[0] + x, portCorner[1] + y).join()
    !== at(a0, a0Corner[0] + x, a0Corner[1] + y).join()).length;
  return { mark: id, pixels: pixels.length, different,
    port_sha256: hash(port, portCorner), a0_sha256: hash(a0, a0Corner) };
}

// The legend window at LEGEND against A0, by kind of pixel.
async function compareLegend(port, a0, layout) {
  const [ox, oy] = LEGEND;
  const { width, height } = layout;
  const owned = new Set();
  const own = (x, y) => owned.add(`${x},${y}`);
  const art = { pixels: 0, different: 0 };
  const exact = (x, y) => {
    own(x, y);
    art.pixels += 1;
    if (!same(port, a0, ox + x, oy + y)) {
      art.different += 1;
      if (art.different <= 12) (art.first ??= []).push([x, y, at(a0, ox + x, oy + y), at(port, ox + x, oy + y)]);
    }
  };
  // FUN_00452630: kinds 1 and 2 alike blit the bitmap keyed (FUN_005fd0f0).
  for (const item of layout.items.filter(({ kind }) => kind === "bitmap")) {
    // The window clips it (the separator 10182 is 210 wide).
    for (const [x, y] of await artPixels(item.id)) {
      if (item.x + x < width && item.y + y < height) exact(item.x + x, item.y + y);
    }
  }
  // FUN_00607740: top and bottom rows, the sides on every second row.
  for (let x = 0; x < width; x += 1) for (const y of [0, 1, height - 2, height - 1]) exact(x, y);
  for (let y = 2; y < height - 2; y += 2) for (const x of [0, 1, width - 2, width - 1]) exact(x, y);
  // The close box (0x277c, 14 by 14) at width - 17, 3.
  for (let y = 3; y < 17; y += 1) for (let x = width - 17; x < width - 3; x += 1) exact(x, y);
  // Each text's ink box (its colour) in its 16-row band: the accepted
  // text-width deviation.
  const texts = layout.items.filter(({ kind }) => kind === "text").map((item) => {
    // Below the frame's two rows; the anti-aliased port glyphs count where
    // every channel is within 60 of the text's colour.
    const band = [item.y === 1 ? 2 : item.x, Math.max(item.y, 2), item.y === 1 ? width - 19 : width - 2, item.y + 17];
    for (let y = band[1]; y < band[3]; y += 1) for (let x = band[0]; x < band[2]; x += 1) own(x, y);
    const ink = (png) => {
      let box = null;
      for (let y = band[1]; y < band[3]; y += 1) {
        for (let x = band[0]; x < band[2]; x += 1) {
          if (at(png, ox + x, oy + y).some((value, c) => Math.abs(value - item.rgb[c]) > 60)) continue;
          box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)]
            : [x, y, x, y];
        }
      }
      return box;
    };
    return { string: item.string, a0: ink(a0), port: ink(port) };
  });
  // The rest is the galaxy view's picture, shaded. Each game's marks
  // differ; a 3-pixel ring round each differing pixel is its mark.
  const rest = [];
  let compared = 0;
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      if (owned.has(`${x},${y}`)) continue;
      compared += 1;
      if (!same(port, a0, ox + x, oy + y)) rest.push([x, y]);
    }
  }
  return { size: [width, height], art, texts, shaded: { compared, different: rest.length } };
}

const nextFrames = (page, count = 2) => page.evaluate((n) => new Promise((resolve) => {
  let left = n;
  const step = () => (--left <= 0 ? resolve() : requestAnimationFrame(step));
  requestAnimationFrame(step);
}), count);
async function stable(page, label) {
  let previous = null;
  for (let attempt = 0; attempt < 20; attempt += 1) {
    await nextFrames(page, 3);
    const bytes = await page.screenshot();
    const hash = createHash("sha256").update(bytes).digest("hex");
    if (hash === previous) {
      await fs.writeFile(path.join(OUT, `${label}.png`), bytes);
      return { png: PNG.sync.read(bytes), sha256: hash };
    }
    previous = hash;
  }
  throw new Error(`${label}: the frame never settled`);
}
async function altDigit(page, digit) {
  await page.keyboard.down("Alt");
  await nextFrames(page);
  await page.keyboard.down(`Digit${digit}`);
  await nextFrames(page);
  await page.keyboard.up(`Digit${digit}`);
  await page.keyboard.up("Alt");
}
// The legend frame's inner white line at (151, 121).
const legendOpen = (png) => at(png, LEGEND[0] + 1, LEGEND[1] + 1).join() === "255,255,255";

await fs.mkdir(OUT, { recursive: true });
const layouts = JSON.parse(await fs.readFile(path.join(ROOT, "web/data/base/gid-legends.json"), "utf8"));
const a0 = async (file) => PNG.sync.read(await fs.readFile(path.join(A0, file)));
const server = await serve(SITE);
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium",
  headless: true,
  args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
const result = { status: "fail" };
try {
  const page = await (await browser.newContext({ viewport: { width: 640, height: 480 } })).newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.goto(`http://127.0.0.1:${server.address().port}/?fixture-code=${GID_A0 | (ALLIANCE << 8)}`);
  await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status === "ready", null,
    { timeout: 30_000 });
  await page.locator("#glcanvas").focus();
  await page.mouse.move(2, 470);
  await page.waitForTimeout(800);

  // CMD-02-C020: Kubindi on Popular Support (A0 B2, day 258).
  const support = await stable(page, "popular-support");
  const b2 = await a0("B2-popular-support-kubindi-explored-unpopulated-day258.png");
  const kubindi = CORNERS.get("Kubindi");
  assert.deepEqual(kubindi, [274, 92]);
  const uninhabited = { a0: await markAt(b2, kubindi), port: await markAt(support.png, kubindi) };
  // The Calaron systems still unexplored keep the unexplored mark beside it.
  uninhabited.neighbours = {};
  for (const name of ["Ithor", "Kessel", "Akrit'tar", "Fwillsving"]) {
    uninhabited.neighbours[name] = { a0: await markAt(b2, CORNERS.get(name)),
      port: await markAt(support.png, CORNERS.get(name)) };
  }
  assert.equal(uninhabited.a0, 10155, "A0 B2: Kubindi's no-side mark");
  assert.equal(uninhabited.port, uninhabited.a0, JSON.stringify(uninhabited));
  uninhabited.diff = await markDiff(support.png, b2, uninhabited.a0, kubindi);
  assert.equal(uninhabited.diff.different, 0, JSON.stringify(uninhabited));
  for (const [name, marks] of Object.entries(uninhabited.neighbours)) {
    assert.ok(marks.a0 === 10158 && marks.port === 10158, `${name}: ${JSON.stringify(marks)}`);
  }
  result.uninhabited = uninhabited;

  // CMD-02-C017: the button's tooltip shows; a press held a second and a
  // click open nothing (A1, A2); a double click opens the legend (A3).
  await page.mouse.move(...BUTTON, { steps: 4 });
  await page.waitForTimeout(1200);
  await stable(page, "legend-button-hover");
  await page.mouse.down();
  await page.waitForTimeout(1000);
  await page.mouse.up();
  await page.waitForTimeout(400);
  const held = await stable(page, "legend-press-held");
  await page.mouse.click(...BUTTON);
  await page.waitForTimeout(600);
  const clicked = await stable(page, "legend-single-click");
  result.single = { held_open: legendOpen(held.png), clicked_open: legendOpen(clicked.png) };
  assert.deepEqual(result.single, { held_open: false, clicked_open: false });

  result.legends = [];
  for (const [index, legend] of LEGENDS.entries()) {
    if (legend.alt) {
      await altDigit(page, legend.alt);
      await page.mouse.move(2, 470);
      const changed = await stable(page, `alt-${legend.alt}`);
      // A4: the display change closes an open legend.
      assert.equal(legendOpen(changed.png), false, `Alt+${legend.alt} left the legend open`);
    }
    await page.mouse.dblclick(...BUTTON);
    await page.waitForTimeout(400);
    await page.mouse.move(2, 470);
    await page.waitForTimeout(400);
    const shown = await stable(page, `legend-${index}`);
    assert.ok(legendOpen(shown.png), `${legend.a0}: the double click opened no legend`);
    const reference = await a0(legend.a0);
    const layout = layouts[String(legend.layout)];
    const comparison = await compareLegend(shown.png, reference, layout);
    result.legends.push({ a0: legend.a0, layout: legend.layout, sha256: shown.sha256, ...comparison });
    assert.equal(comparison.art.different, 0, `${legend.a0}: ${JSON.stringify(comparison.art)}`);
    for (const text of comparison.texts) {
      assert.ok(text.a0 && text.port, `${legend.a0}: text ${text.string} has no ink`);
      // Left and top within a pixel; the right edge follows the font's
      // width (the accepted text deviation).
      assert.ok(Math.abs(text.a0[1] - text.port[1]) <= 1 && Math.abs(text.a0[3] - text.port[3]) <= 1,
        `${legend.a0}: text ${text.string} ${JSON.stringify(text)}`);
    }
    assert.ok(comparison.shaded.different / comparison.shaded.compared <= 0.05,
      `${legend.a0}: shaded galaxy ${JSON.stringify(comparison.shaded)}`);

    if (legend.alt === "6") {
      // CMD-02-C023: Averam with Tallon on a mission (A0 C2, day 245) and
      // the unexplored systems' "None" marks (A6), with the legend closed.
      // The close box; the release must land a frame before the pointer
      // leaves (playwright release frame).
      await page.mouse.move(LEGEND[0] + layout.width - 10, LEGEND[1] + 10, { steps: 2 });
      await nextFrames(page);
      await page.mouse.down();
      await nextFrames(page);
      await page.mouse.up();
      await page.waitForTimeout(300);
      await page.mouse.move(2, 470);
      await page.waitForTimeout(400);
      const personnel = await stable(page, "active-personnel");
      assert.equal(legendOpen(personnel.png), false, "the close box left the legend open");
      const c2 = await a0("C2-active-personnel-tallon-on-mission-at-averam-day245.png");
      const a6 = await a0("A6-active-personnel-after-alt6.png");
      const averam = CORNERS.get("Averam");
      const mission = { a0: await markAt(c2, averam), port: await markAt(personnel.png, averam),
        a0_before: await markAt(a6, averam) };
      assert.deepEqual([mission.a0, mission.a0_before], [10146, 10149], "A0 C2, A6: Averam's marks");
      assert.equal(mission.port, mission.a0, JSON.stringify(mission));
      mission.diff = await markDiff(personnel.png, c2, mission.a0, averam);
      assert.equal(mission.diff.different, 0, JSON.stringify(mission));
      // Every system the port shows unexplored on Popular Support shows
      // the no-side "None" mark here, as all of A0's 68 do (A6).
      const unexplored = [];
      for (const [name, corner] of CORNERS) {
        if (await markAt(support.png, corner) === 10158) unexplored.push([name, await markAt(personnel.png, corner)]);
      }
      mission.unexplored = unexplored.length;
      mission.unexplored_not_none = unexplored.filter(([, id]) => id !== 10154);
      assert.ok(unexplored.length > 0);
      assert.deepEqual(mission.unexplored_not_none, []);
      result.mission = mission;
      await page.mouse.dblclick(...BUTTON);
      await page.waitForTimeout(400);
    }
  }

  // CMD-02-C024: the both-sides mark on Idle Fleets.
  const pinned = {};
  for (const [key, [file, sha]] of Object.entries(A0_BOTH_SIDES_FILES)) {
    const bytes = await fs.readFile(path.join(A0_BOTH_SIDES, file));
    assert.equal(createHash("sha256").update(bytes).digest("hex").slice(0, 12), sha, `A0 ${file}`);
    pinned[key] = PNG.sync.read(bytes);
  }
  const coruscant = CORNERS.get("Coruscant");
  assert.deepEqual(coruscant, [226, 150]);
  const both = { a0: await markAt(pinned.both, coruscant), a0_before: await markAt(pinned.before, coruscant) };
  assert.deepEqual([both.a0, both.a0_before], [10160, 10150], "A0 a-02, a-01: Coruscant's marks");
  const fleets = await (await browser.newContext({ viewport: { width: 640, height: 480 } })).newPage();
  fleets.on("pageerror", (error) => errors.push(String(error)));
  await fleets.goto(`http://127.0.0.1:${server.address().port}/?fixture-code=${FLEETS_BOTH_SIDES | (ALLIANCE << 8)}`);
  await fleets.waitForFunction(() => window.__openRebellionInterfaceReady?.status === "ready", null,
    { timeout: 30_000 });
  const ready = await fleets.evaluate(() => window.__openRebellionInterfaceReady);
  assert.equal(ready.mode, "Idle Fleets", JSON.stringify(ready));
  assert.deepEqual(ready.primary_gid_marks, [10160], JSON.stringify(ready));
  await fleets.locator("#glcanvas").focus();
  await fleets.mouse.move(2, 470);
  await fleets.waitForTimeout(800);
  const shown = await stable(fleets, "idle-fleets-both-sides");
  // FUN_00425d00's corner for the primary system's SYSTEMSD point, and
  // the port's drawn point less half the 17-pixel cell.
  const corner = [((ready.primary_system_x * 607) >> 10) + 21, ((ready.primary_system_y * 437) >> 10) + 25];
  both.port_corner = corner;
  both.drawn_corner = [Math.round(ready.primary_screen_x - 8.5), Math.round(ready.primary_screen_y - 8.5)];
  assert.deepEqual(both.drawn_corner, corner, JSON.stringify(both));
  both.port = await markAt(shown.png, corner);
  assert.equal(both.port, 10160, JSON.stringify(both));
  both.diff = await markDiffAt(shown.png, corner, pinned.both, coruscant, 10160);
  assert.equal(both.diff.different, 0, JSON.stringify(both));
  assert.equal(both.diff.port_sha256, both.diff.a0_sha256, JSON.stringify(both));
  both.sha256 = shown.sha256;
  result.both_sides = both;

  assert.deepEqual(errors, []);
  result.status = "pass";
} catch (error) {
  result.error = String(error.stack || error);
} finally {
  await browser.close();
  server.close();
}
const wasm = await fs.readFile(path.join(SITE, "open-rebellion-test.wasm"));
result.wasm_sha256 = createHash("sha256").update(wasm).digest("hex");
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: result.status, error: result.error }, null, 2));
if (result.status !== "pass") process.exitCode = 1;
