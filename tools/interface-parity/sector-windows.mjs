// The sector windows' placement against A0 (sector_window.rs,
// FUN_00429ce0, FUN_004593e0).
//
// Replays the A0 session `.artifacts/a0-vm/sector-windows` (--a0) in the
// Alliance Popular Support fixture (code 5): a system in Sesswenna opens
// its sector on the left half, one in Sumitra opens on the right half,
// Sesswenna's side switch moves it to the right half and closes Sumitra
// there, a system in Churba then opens on the freed left half, Churba
// closes, and Sesswenna's system again opens no second window. After
// each step it compares, pixel for pixel with the A0 frame, each sector
// window's title-bar buttons (side switch and close; there is no minimize
// box, FUN_004593e0) and the window's outer frame (its fixed place), and
// the sector name's ink box within the accepted text deviation. In Churba
// (04-left) the unexplored planets' pictures and the rows below them must
// match A0 exactly: no map-item marker. The
// windows' contents show each game's state and are left out. Last, the
// whole ten-system Moddell window is compared with A0
// 2026-10-28-nav/sector-a (see the step's comment).
//
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node sector-windows.mjs --a0=.artifacts/a0-vm/sector-windows [--out=DIR]
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
const args = Object.fromEntries(process.argv.slice(2).map((arg) => {
  const [key, value = "true"] = arg.replace(/^--/, "").split("=");
  return [key, value];
}));
assert.ok(args.a0, "--a0=DIR (the A0 sector-windows captures) is required");
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `sector-windows-${new Date().toISOString().replace(/[:.]/g, "-")}`));
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".orpk": "application/octet-stream", ".webm": "video/webm" };
const POPULAR_SUPPORT = 5;
const ALLIANCE = 1;

// The two halves' windows, measured on A0 02-second.png: x 60 and 300,
// 235 by 360 from y 35.
const HALVES = { left: 60, right: 300 };
// The side switch and close box at (204, 2) and (218, 2), 14 by 14; the
// frame's top, bottom and sides (FUN_00607740); the title from (2, 2).
const window = (x) => ({
  buttons: [x + 204, 37, 28, 14],
  frame: [[x, 35, 235, 2], [x, 393, 235, 2], [x, 35, 2, 360], [x + 233, 35, 2, 360]],
  title: [x + 20, 37, 180, 20],
});
// Churba's unexplored planets on the left half (A0 04-left): each 37 by 37
// picture and the 4 rows below it, where FUN_0045bbb0 blits the system's
// map-item bitmap, which an unexplored system's item does not hold
// (FUN_0042b330). Only the planets clear of the galaxy's own markers
// (each game's state) behind the window.
// The status tracks' colours (sector_window::status_tracks), as rgb()
// numbers.
const TRACK_COLOURS = new Set([0x808080, 0xffffff, 0x0000ff, 0xffff00, 0xf95c0f, 0xff0000, 0x00ff00]);
const CHURBA_PLANETS = { Lelmra: [70, 58], Deyer: [159, 132], Gentes: [85, 157], Allyuen: [70, 241],
  Hoth: [139, 241], Storthus: [93, 315] };
// Churba's ten names, white (an unexplored system's), their A0 ink boxes
// (left, top, right, bottom inclusive).
const CHURBA_NAMES = { Lelmra: [76, 110, 106, 117], Tokmia: [136, 117, 166, 124], Anoat: [224, 152, 251, 159],
  Deyer: [167, 184, 194, 193], Gentes: [89, 209, 122, 216], Bespin: [201, 258, 232, 267],
  Allyuen: [74, 293, 108, 302], Hoth: [150, 293, 170, 300], "New Cov": [204, 332, 247, 339],
  Storthus: [95, 367, 134, 374] };
function whiteBox(png, [l, t, r, b]) {
  let box = null;
  for (let y = t - 3; y <= b + 3; y += 1) {
    for (let x = l - 8; x <= r + 8; x += 1) {
      const i = (y * png.width + x) * 4;
      if (png.data[i] > 200 && png.data[i + 1] > 200 && png.data[i + 2] > 200) {
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)]
          : [x, y, x, y];
      }
    }
  }
  return box;
}
// The galaxy view's caption under each window (FUN_00425d00 draws it into
// the view's picture, FUN_004593e0 copies that under the window, shaded):
// A0 shows "Popular Support" (255, 0, 0) there as (91, 67, 51), from x 252
// to the left window's buttons and from the right window's edge to x 342.
// Its ink box, as reddish pixels (the port's glyphs are anti-aliased
// Liberation), in each half's title rows left of its buttons.
const CAPTION_COLUMNS = { left: [62, 264], right: [302, 504] };
function captionBox(png, [x0, x1]) {
  let box = null;
  for (let y = 37; y < 58; y += 1) {
    for (let x = x0; x < x1; x += 1) {
      const i = (y * png.width + x) * 4;
      const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
      if (r >= 75 && r - b >= 25 && r - g >= 12) {
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)]
          : [x, y, x, y];
      }
    }
  }
  return box;
}
// [A0 frame, the click before it, the halves holding a window]
const STEPS = [
  ["01-first", [252, 162], ["left"]],
  ["02-second", [470, 240], ["left", "right"]],
  ["03-flip", [270, 44], ["right"]],
  ["04-left", [115, 215], ["left", "right"]],
  // Churba's close box, then Sesswenna's system again: its sector already
  // has a window (right), so no second one opens (FUN_00429ce0).
  ["05-closed-left", [284, 44], ["right"]],
  ["06-same-sector", [252, 162], ["right"]],
  // Churba again, then a third sector with both halves held: the System
  // Finder (F2) on Kashyyyk opens Sumitra, which replaces the right half's
  // window (Sesswenna) and leaves Churba (FUN_00429ce0).
  ["07-both", [115, 215], ["left", "right"]],
  ["09-third", { keys: ["F2", "type:Kashyyyk", "Enter"] }, ["left", "right"]],
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

const rgb = (png, x, y) => {
  const i = (y * png.width + x) * 4;
  return (png.data[i] << 16) | (png.data[i + 1] << 8) | png.data[i + 2];
};
// The pixels the window's own art covers: not the buttons' keyed corner
// (their first DIB pixel, bottom left), nor the rows between the side
// tiles, where the shaded galaxy (each game's state) shows.
function covered(x, y, left) {
  if (y === 50 && (x === left + 204 || x === left + 218)) return false;
  const side = x < left + 2 || x >= left + 233;
  return !(side && y >= 37 && y < 393 && (y - 35) % 2 === 1);
}
function differing(port, a0, [rx, ry, rw, rh], left) {
  let count = 0;
  for (let y = ry; y < ry + rh; y += 1) {
    for (let x = rx; x < rx + rw; x += 1) {
      if (covered(x, y, left) && rgb(port, x, y) !== rgb(a0, x, y)) count += 1;
    }
  }
  return count;
}
// The pixels of `region` outside `masks` that differ, grouped into patches
// (pixels within 2 of each other): their count, total and largest size.
function patchesOutside(port, a0, [rx, ry, rw, rh], masks) {
  const masked = (x, y) => masks.some(([mx, my, mw, mh]) => x >= mx && y >= my && x < mx + mw && y < my + mh);
  const rest = new Set();
  let compared = 0;
  for (let y = ry; y < ry + rh; y += 1) {
    for (let x = rx; x < rx + rw; x += 1) {
      if (masked(x, y)) continue;
      compared += 1;
      if (rgb(port, x, y) !== rgb(a0, x, y)) rest.add(`${x},${y}`);
    }
  }
  const patches = [];
  while (rest.size) {
    const [first] = rest;
    rest.delete(first);
    const stack = [first.split(",").map(Number)];
    let [l, t, r, b, count] = [...stack[0], ...stack[0], 0];
    while (stack.length) {
      const [x, y] = stack.pop();
      count += 1;
      [l, t, r, b] = [Math.min(l, x), Math.min(t, y), Math.max(r, x), Math.max(b, y)];
      for (let dy = -2; dy <= 2; dy += 1) {
        for (let dx = -2; dx <= 2; dx += 1) {
          const key = `${x + dx},${y + dy}`;
          if (rest.delete(key)) stack.push([x + dx, y + dy]);
        }
      }
    }
    patches.push([l, t, r - l + 1, b - t + 1, count]);
  }
  return { compared, patches: patches.length, pixels: patches.reduce((sum, patch) => sum + patch[4], 0),
    largest: patches.reduce((most, patch) => [Math.max(most[0], patch[2]), Math.max(most[1], patch[3])], [0, 0]) };
}
// The title's yellow ink box.
function inkBox(png, [x0, y0, w, h]) {
  let box = null;
  for (let y = y0; y < y0 + h; y += 1) {
    for (let x = x0; x < x0 + w; x += 1) {
      const i = (y * png.width + x) * 4;
      const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
      if (r > 180 && g > 180 && b < 90) {
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)]
          : [x, y, x, y];
      }
    }
  }
  return box;
}

await fs.mkdir(OUT, { recursive: true });
const server = await serve(SITE);
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium",
  headless: true,
  args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
const result = { status: "fail", steps: [] };
try {
  const page = await (await browser.newContext({ viewport: { width: 640, height: 480 } })).newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.goto(`http://127.0.0.1:${server.address().port}/?fixture-code=${POPULAR_SUPPORT | (ALLIANCE << 8)}`);
  await page.waitForFunction(() => Boolean(window.__openRebellionInterfaceReady), null, { timeout: 30_000 });
  await page.locator("#glcanvas").focus();
  await page.waitForTimeout(800);
  for (const [frame, click, halves] of STEPS) {
    if (Array.isArray(click)) {
      await page.mouse.move(...click, { steps: 4 });
      await page.waitForTimeout(150);
      await page.mouse.down();
      await page.waitForTimeout(120);
      await page.mouse.up();
    } else {
      for (const key of click.keys) {
        if (key.startsWith("type:")) await page.keyboard.type(key.slice(5), { delay: 60 });
        else await page.keyboard.press(key);
        await page.waitForTimeout(700);
      }
    }
    await page.waitForTimeout(600);
    await page.mouse.move(2, 470, { steps: 4 });
    await page.waitForTimeout(400);
    const bytes = await page.screenshot({ path: path.join(OUT, `${frame}.png`) });
    const port = PNG.sync.read(bytes);
    const a0 = PNG.sync.read(await fs.readFile(path.join(args.a0, `${frame}.png`)));
    const step = { frame, sha256: createHash("sha256").update(bytes).digest("hex"), halves: {} };
    for (const half of ["left", "right"]) {
      const shape = window(HALVES[half]);
      const regions = [shape.buttons, ...shape.frame];
      step.halves[half] = {
        expected: halves.includes(half),
        differing: regions.map((region) => differing(port, a0, region, HALVES[half])),
        ink: { a0: inkBox(a0, shape.title), port: inkBox(port, shape.title) },
        caption: { a0: captionBox(a0, CAPTION_COLUMNS[half]), port: captionBox(port, CAPTION_COLUMNS[half]) },
      };
    }
    if (frame === "04-left") {
      step.planets = Object.fromEntries(Object.entries(CHURBA_PLANETS)
        .map(([name, [x, y]]) => [name, differing(port, a0, [x, y, 37, 41], 0)]));
      step.names = Object.fromEntries(Object.entries(CHURBA_NAMES)
        .map(([name, box]) => [name, { a0: box, port: whiteBox(port, box) }]));
    }
    if (frame === "04-left") {
      // The whole Churba window (60, 35, 235 by 360) against A0. Its ten
      // systems are unexplored in both games, so beyond the parts above
      // only the shaded galaxy behind may differ: the galaxy markers under
      // the window are each game's own. The title row (title and shaded
      // caption, checked by ink) and the ten names (Liberation's glyphs)
      // are left out. Each differing patch must be a marker's size or two
      // markers' stacked (a cross is 7 by 7, a diamond 9 by 9); a wrong
      // shade, background or picture would make one large patch.
      const masks = [[62, 37, 231, 21]];
      for (const [l, t, r, b] of Object.values(CHURBA_NAMES)) masks.push([l - 8, t - 3, r - l + 17, b - t + 7]);
      step.whole = patchesOutside(port, a0, [62, 37, 231, 356], masks);
    }
    result.steps.push(step);
    if (step.whole) {
      assert.ok(step.whole.largest[0] <= 16 && step.whole.largest[1] <= 26 && step.whole.pixels < 700,
        `04-left: the whole Churba window ${JSON.stringify(step.whole)}`);
    }
    if (step.planets) {
      assert.ok(Object.values(step.planets).every((count) => count === 0),
        `04-left planets: ${JSON.stringify(step.planets)}`);
      // Each name in white, centred within a pixel and a half and topped
      // within one, as wide within 7 %: the port sets each character at
      // GDI's pen (Arial's advances and kerning at 11 ppem,
      // theme::gdi_pen_positions); the glyphs themselves are Liberation's.
      for (const [name, entry] of Object.entries(step.names)) {
        const { a0: a, port: p } = entry;
        assert.ok(p, `${name}: no white name`);
        assert.ok(Math.abs((a[0] + a[2]) / 2 - (p[0] + p[2]) / 2) <= 1.5 && Math.abs(a[1] - p[1]) <= 1
          && Math.abs(a[3] - p[3]) <= 1, `${name}: ink ${p} vs A0 ${a}`);
        entry.width_ratio = Number(((p[2] - p[0] + 1) / (a[2] - a[0] + 1)).toFixed(3));
        assert.ok(entry.width_ratio >= 0.93 && entry.width_ratio <= 1.07, `${name}: width ratio ${entry.width_ratio}`);
      }
      // A click selects Lelmra and leaves its picture and name as they
      // were: A0 2026-10-28-nav/adega-selected shows a selected planet
      // with no frame and its name still white (FUN_004593e0).
      const [lx, ly] = CHURBA_PLANETS.Lelmra;
      await page.mouse.move(lx + 18, ly + 18, { steps: 4 });
      await page.waitForTimeout(150);
      await page.mouse.down();
      await page.waitForTimeout(120);
      await page.mouse.up();
      await page.waitForTimeout(500);
      await page.mouse.move(2, 470, { steps: 4 });
      await page.waitForTimeout(400);
      const selected = PNG.sync.read(await page.screenshot({ path: path.join(OUT, "04-left-lelmra-selected.png") }));
      const [nl, nt, nr, nb] = CHURBA_NAMES.Lelmra;
      step.selection = {
        planet: differing(selected, port, [lx, ly, 37, 41], 0),
        name: differing(selected, port, [nl - 4, nt - 2, nr - nl + 9, nb - nt + 5], 0),
      };
      assert.deepEqual(step.selection, { planet: 0, name: 0 }, `a selected planet changes: ${JSON.stringify(step.selection)}`);
    }
    for (const [half, { expected, differing: counts, ink, caption }] of Object.entries(step.halves)) {
      if (!expected) {
        assert.equal(ink.port, null, `${frame} ${half}: the port shows a window A0 does not`);
        continue;
      }
      // The shaded caption: left, top and bottom within a pixel, the right
      // end within 3 (the accepted text width deviation).
      {
        const [a, p] = [caption.a0, caption.port];
        assert.ok(a && p && [0, 1, 3].every((k) => Math.abs(a[k] - p[k]) <= 1) && Math.abs(a[2] - p[2]) <= 3,
          `${frame} ${half}: shaded caption ${JSON.stringify(caption)}`);
      }
      assert.deepEqual(counts, counts.map(() => 0), `${frame} ${half}: ${JSON.stringify(step.halves[half])}`);
      const [a, p] = [ink.a0, ink.port];
      assert.ok(a && p, `${frame} ${half}: no title ink`);
      assert.ok(Math.abs((a[0] + a[2]) / 2 - (p[0] + p[2]) / 2) <= 1.5 && Math.abs(a[1] - p[1]) <= 1
        && Math.abs(a[3] - p[3]) <= 1, `${frame} ${half}: title ink ${p} vs A0 ${a}`);
    }
  }
  // The whole ten-system Moddell window against A0 2026-10-28-nav/sector-a
  // (a new Alliance game, day 8; Moddell on the left half). The fixture's
  // galaxy is another game's: Vjun is the Empire's there and holds its
  // fleet and marks, the cursor rests on Adega, and the galaxy's own
  // markers show through the shaded background. Everything else must
  // match: the frame and its buttons, the title, nine planets' pictures
  // and their white names.
  const moddell = await (await browser.newContext({ viewport: { width: 640, height: 480 } })).newPage();
  moddell.on("pageerror", (error) => errors.push(String(error)));
  await moddell.goto(`http://127.0.0.1:${server.address().port}/?fixture-code=${POPULAR_SUPPORT | (ALLIANCE << 8)}`);
  await moddell.waitForFunction(() => Boolean(window.__openRebellionInterfaceReady), null, { timeout: 30_000 });
  await moddell.locator("#glcanvas").focus();
  await moddell.waitForTimeout(800);
  for (const key of ["F2", "type:Adega", "Enter"]) {
    if (key.startsWith("type:")) await moddell.keyboard.type(key.slice(5), { delay: 60 });
    else await moddell.keyboard.press(key);
    await moddell.waitForTimeout(700);
  }
  await moddell.mouse.move(2, 470, { steps: 4 });
  await moddell.waitForTimeout(600);
  const port = PNG.sync.read(await moddell.screenshot({ path: path.join(OUT, "moddell.png") }));
  const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/2026-10-28-nav/sector-a.png")));
  // A0's name ink boxes; each picture is 37 by 41, centred over its name,
  // 52 rows above the name's top (as Churba's in 04-left).
  const names = { Agrilat: [202, 117, 231, 126], Adega: [136, 135, 166, 144], Basilisk: [83, 184, 117, 191],
    Khuiumin: [139, 240, 180, 247], Endor: [204, 247, 230, 254], Annaj: [78, 265, 104, 274],
    Pzob: [134, 307, 156, 314], "Hozrel XI": [184, 349, 225, 356], "Gandolo IV": [95, 374, 146, 381] };
  const picture = ([l, t, r]) => [Math.round((l + r + 1) / 2 - 18.5), t - 52, 37, 41];
  // Adega's picture left of A0's cursor (x 160..), Endor's right of the
  // galaxy markers behind its keyed left edge (x ..208).
  const parts = { Adega: [133, 83, 27, 41], Endor: [209, 195, 27, 41] };
  const shape = window(HALVES.left);
  const whole = {
    frame: [shape.buttons, ...shape.frame].map((region) => differing(port, a0, region, HALVES.left)),
    title: { a0: inkBox(a0, shape.title), port: inkBox(port, shape.title) },
    planets: Object.fromEntries(Object.entries(names).map(([name, box]) =>
      [name, differing(port, a0, parts[name] ?? picture(box), 0)])),
    names: Object.fromEntries(Object.entries(names).map(([name, box]) => [name, { a0: box, port: whiteBox(port, box) }])),
  };
  // The rest of the window: the shaded galaxy. Each differing patch must
  // be a marker's size (a cross or a diamond, at most 16 by 20).
  const masks = [[62, 37, 231, 21], [205, 120, 85, 80], [158, 105, 30, 32]];
  for (const box of Object.values(names)) {
    const [l, t, r, b] = box;
    masks.push([l - 8, t - 3, r - l + 17, b - t + 7], picture(box));
  }
  const masked = (x, y) => masks.some(([mx, my, mw, mh]) => x >= mx && y >= my && x < mx + mw && y < my + mh);
  const rest = new Set();
  for (let y = 37; y < 393; y += 1) {
    for (let x = 62; x < 293; x += 1) {
      if (!masked(x, y) && rgb(port, x, y) !== rgb(a0, x, y)) rest.add(`${x},${y}`);
    }
  }
  const patches = [];
  while (rest.size) {
    const [first] = rest;
    rest.delete(first);
    const stack = [first.split(",").map(Number)];
    let [l, t, r, b, count] = [...stack[0], ...stack[0], 0];
    while (stack.length) {
      const [x, y] = stack.pop();
      count += 1;
      [l, t, r, b] = [Math.min(l, x), Math.min(t, y), Math.max(r, x), Math.max(b, y)];
      for (let dy = -2; dy <= 2; dy += 1) {
        for (let dx = -2; dx <= 2; dx += 1) {
          const key = `${x + dx},${y + dy}`;
          if (rest.delete(key)) stack.push([x + dx, y + dy]);
        }
      }
    }
    patches.push([l, t, r - l + 1, b - t + 1, count]);
  }
  whole.background = { patches: patches.length, pixels: patches.reduce((sum, patch) => sum + patch[4], 0),
    largest: patches.reduce((most, patch) => [Math.max(most[0], patch[2]), Math.max(most[1], patch[3])], [0, 0]) };
  result.moddell = whole;
  assert.deepEqual(whole.frame, whole.frame.map(() => 0), `Moddell frame: ${JSON.stringify(whole.frame)}`);
  {
    const { a0: a, port: p } = whole.title;
    assert.ok(a && p && Math.abs((a[0] + a[2]) / 2 - (p[0] + p[2]) / 2) <= 1.5 && Math.abs(a[1] - p[1]) <= 1,
      `Moddell title ink ${p} vs A0 ${a}`);
  }
  assert.ok(Object.values(whole.planets).every((count) => count === 0), `Moddell planets: ${JSON.stringify(whole.planets)}`);
  for (const [name, { a0: a, port: p }] of Object.entries(whole.names)) {
    assert.ok(p, `Moddell ${name}: no white name`);
    const ratio = (p[2] - p[0] + 1) / (a[2] - a[0] + 1);
    assert.ok(Math.abs((a[0] + a[2]) / 2 - (p[0] + p[2]) / 2) <= 1.5 && Math.abs(a[1] - p[1]) <= 1
      && Math.abs(a[3] - p[3]) <= 1 && ratio >= 0.93 && ratio <= 1.07, `Moddell ${name}: ink ${p} vs A0 ${a}`);
  }
  assert.ok(whole.background.largest[0] <= 16 && whole.background.largest[1] <= 20,
    `Moddell background patch larger than a marker: ${JSON.stringify(whole.background)}`);

  // Uninhabited planets (CMD-03 "uninhabited") against A0
  // headquarters/00-moddell (the same new Alliance game later; Moddell on
  // the left half): Agrilat, Khuiumin, Endor and Annaj are explored
  // and show their energy and raw-material tracks but no support track,
  // which FUN_0045baf0 shows only for a populated system (+0x88 bit 0)
  // with a side (+0x24 & 0xc0). Fixture 102 (Scenario::ModdellUninhabited,
  // 101) gives them A0's slots, no facility and no side. Between each
  // picture and its name (the name's top less 14 to less 2: the energy,
  // raw-material and support rows), every pixel either game draws in a
  // track colour must be the other's too (the shaded galaxy behind differs
  // between the two games, as in the Moddell case above). Adega, also
  // uninhabited there, is left out: A0's cursor hides its tracks' ends.
  // The track colours: the ticks 0x808080
  // (FUN_0060e3b0), energy white and blue, raw yellow and orange, support
  // red and green.
  const uninhabited = await (await browser.newContext({ viewport: { width: 640, height: 480 } })).newPage();
  uninhabited.on("pageerror", (error) => errors.push(String(error)));
  await uninhabited.goto(`http://127.0.0.1:${server.address().port}/?fixture-code=${102 | (ALLIANCE << 8)}`);
  await uninhabited.waitForFunction(() => Boolean(window.__openRebellionInterfaceReady), null, { timeout: 30_000 });
  await uninhabited.locator("#glcanvas").focus();
  await uninhabited.waitForTimeout(800);
  for (const key of ["F2", "type:Adega", "Enter"]) {
    if (key.startsWith("type:")) await uninhabited.keyboard.type(key.slice(5), { delay: 60 });
    else await uninhabited.keyboard.press(key);
    await uninhabited.waitForTimeout(700);
  }
  await uninhabited.mouse.move(2, 470, { steps: 4 });
  await uninhabited.waitForTimeout(600);
  const portUninhabited = PNG.sync.read(await uninhabited.screenshot({ path: path.join(OUT, "moddell-uninhabited.png") }));
  const a0Uninhabited = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/headquarters/00-moddell.png")));
  const tracks = {};
  for (const name of ["Agrilat", "Khuiumin", "Endor", "Annaj"]) {
    const [l, t, r] = names[name];
    const [x] = picture([l, t, r]);
    const width = 37;
    let differ = 0;
    let drawn = 0;
    for (let y = t - 14; y < t - 1; y += 1) {
      for (let column = x; column < x + width; column += 1) {
        const [p, a] = [rgb(portUninhabited, column, y), rgb(a0Uninhabited, column, y)];
        if (!TRACK_COLOURS.has(p) && !TRACK_COLOURS.has(a)) continue;
        if (TRACK_COLOURS.has(a)) drawn += 1;
        if (p !== a) differ += 1;
      }
    }
    assert.ok(drawn > 50, `${name}: A0 shows no tracks (${drawn})`);
    tracks[name] = differ;
  }
  result.uninhabited = tracks;
  assert.ok(Object.values(tracks).every((count) => count === 0), `Moddell uninhabited tracks: ${JSON.stringify(tracks)}`);
  assert.deepEqual(errors, []);
  result.status = "pass";
} catch (error) {
  result.error = String(error.stack || error);
} finally {
  await browser.close();
  server.close();
}
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: result.status, steps: result.steps.map(({ frame, halves }) => ({ frame, halves })),
  error: result.error }, null, 2));
if (result.status !== "pass") process.exitCode = 1;
