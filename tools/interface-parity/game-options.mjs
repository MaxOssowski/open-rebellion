// The Game Options window against A0 (ghidra/notes/game-options.md,
// crates/rebellion-render/src/game_options.rs).
//
// Opens Game Options (F1) from the Empire Fleet Finder fixture, checks the
// empty list (no names, no crests), types "test" into the first row's name
// box and presses its Save button (the original row, FUN_00406b80 0x6f),
// reopens the window and compares it with A0 tooltip/200-options.png (an
// Empire game from the galaxy, six saves, the first named "test"):
// - every pixel outside the texts, the five rows A0 has filled and the port
//   has not, the first row's crest and the two slider thumbs must match;
// - the first row's crest (the port's Empire save) must match A0's fourth
//   row (an Empire save, COMMON 10055) pixel for pixel;
// - each slider must match A0 apart from at most two runs of 12 columns
//   (the two thumbs, at different volumes);
// - each text's ink box must match A0's within the accepted text deviation
//   (2026-10-30): left or centre and top and bottom within 1 pixel, width
//   within 15 per cent. The version label is the port's own (port:) and is
//   left out.
// It then drags the music slider to slider x 100 and checks the thumb
// centre at floor(201 * 49 / 100) = 98 (FUN_00604950, FUN_00604a50),
// reloads the page and checks the slider kept its place (the stored
// settings).
//
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node game-options.mjs --a0=.artifacts/a0-vm/tooltip [--out=DIR]
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
assert.ok(args.a0, "--a0=DIR (the A0 tooltip captures) is required");
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `game-options-${new Date().toISOString().replace(/[:.]/g, "-")}`));
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".orpk": "application/octet-stream", ".webm": "video/webm" };
const FLEET_FINDER = 53;
const EMPIRE = 2;

const green = ([r, g, b]) => g >= 100 && r < 60 && b < 60;
const darkGreen = ([r, g, b]) => g >= 90 && g <= 140 && r < 40 && b < 40;
const white = (rgb) => Math.min(...rgb) >= 200;
const black = (rgb) => Math.max(...rgb) < 40;
// FUN_004084b0 headings and FUN_00601620 text objects: [name, rect, ink,
// centred].
const TEXTS = [
  ["Saved Games", [58, 37, 241, 21], green, true],
  ["Sound Options", [362, 37, 241, 21], green, true],
  ["Tactical Display Options", [363, 273, 239, 21], green, true],
  ["Play Music", [376, 79, 183, 33], darkGreen, false],
  ["Off", [581, 79, 31, 33], darkGreen, false],
  ...["Show Starfield", "Show Planet", "Show Pyrotechnics", "Use High Detail Models", "Display Holocube"]
    .flatMap((label, k) => [[label, [394, 313 + 27 * k, 183, 16], green, false],
      [`On ${k + 1}`, [577, 313 + 27 * k, 31, 16], green, false]]),
  ["test", [119, 81, 160, 20], white, false],
// The text objects draw with DT_NOCLIP (FUN_00601ce0): their descenders
// reach below their rows, so each region takes three more rows.
].map(([name, [x, y, w, h], ink, centred]) => [name, [x, y, w, h + 3], ink, centred]);
const VERSION = [25, 437, 310, 18];
const ROW_Y = [81, 123, 165, 207, 249, 291];
const SLIDERS = [[392, 130, 201, 54], [392, 190, 201, 54]];
// A0's pointer rests over the Return/Load gap (200-options.png).
const POINTER = [318, 236, 24, 30];

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

const inside = (x, y, [rx, ry, rw, rh]) => x >= rx && x < rx + rw && y >= ry && y < ry + rh;
const rgb = (png, x, y) => {
  const i = (y * png.width + x) * 4;
  return [png.data[i], png.data[i + 1], png.data[i + 2]];
};
const same = (a, ax, ay, b, bx, by) => rgb(a, ax, ay).every((v, i) => v === rgb(b, bx, by)[i]);
function inkBox(png, [x0, y0, w, h], ink) {
  let box = null;
  for (let y = y0; y < y0 + h; y += 1) {
    for (let x = x0; x < x0 + w; x += 1) {
      if (!ink(rgb(png, x, y))) continue;
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)]
        : [x, y, x, y];
    }
  }
  return box;
}
// The rows A0 has filled and the port has not, and the first row's crest.
const rowRects = ROW_Y.slice(1).map((y) => [83, y - 1, 247, 23]);
const CREST = [85, 83, 26, 19];

await fs.mkdir(OUT, { recursive: true });
const a0Bytes = await fs.readFile(path.join(args.a0, "200-options.png"));
const a0 = PNG.sync.read(a0Bytes);
const server = await serve(SITE);
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium",
  headless: true,
  args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
const result = { status: "fail", a0: { file: "200-options.png", sha256: createHash("sha256").update(a0Bytes).digest("hex") } };
try {
  const page = await (await browser.newContext({ viewport: { width: 640, height: 480 } })).newPage();
  const errors = [];
  const consoleLog = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  page.on("console", (message) => consoleLog.push(message.text()));
  const shot = async (name) => {
    const bytes = await page.screenshot({ path: path.join(OUT, `${name}.png`) });
    result.shots = { ...(result.shots ?? {}), [name]: createHash("sha256").update(bytes).digest("hex") };
    return PNG.sync.read(bytes);
  };
  await page.goto(`http://127.0.0.1:${server.address().port}/?fixture-code=${FLEET_FINDER | (EMPIRE << 8)}`);
  await page.waitForFunction(() => (window.__openRebellionInterfaceSystemFinders || []).length > 0,
    null, { timeout: 30_000 });
  await page.locator("#glcanvas").focus();
  await page.waitForTimeout(500);
  await page.keyboard.press("F1");
  await page.waitForTimeout(1000);
  await page.mouse.move(330, 250);
  await page.waitForTimeout(300);

  // The empty list: no name and no crest in any row (FUN_00407180).
  const empty = await shot("empty");
  const emptyInk = ROW_Y.map((y) => inkBox(empty, [119, y, 160, 20], white));
  assert.deepEqual(emptyInk, [null, null, null, null, null, null], "an empty row shows no name");
  const crestColours = (png, y) => {
    let n = 0;
    for (let yy = y + 2; yy < y + 21; yy += 1) {
      for (let x = 85; x < 111; x += 1) if (green(rgb(png, x, yy)) || rgb(png, x, yy)[0] > 180) n += 1;
    }
    return n;
  };
  assert.deepEqual(ROW_Y.map((y) => crestColours(empty, y)), [0, 0, 0, 0, 0, 0], "an empty row shows no crest");

  // Save "test" from the first row.
  await page.mouse.click(180, 91);
  await page.keyboard.type("test");
  await page.waitForTimeout(200);
  await page.mouse.click(55, 91);
  await page.waitForTimeout(1500);
  assert.ok(consoleLog.some((line) => line.includes("save_state_fingerprint slot=0")), "the first row's Save saves slot 0");
  await page.keyboard.press("F1");
  await page.waitForTimeout(1000);
  await page.mouse.move(330, 250);
  await page.waitForTimeout(300);
  const port = await shot("saved");

  let compared = 0;
  const differing = [];
  for (let y = 0; y < 480; y += 1) {
    for (let x = 0; x < 640; x += 1) {
      if (TEXTS.some(([, rect]) => inside(x, y, rect)) || inside(x, y, VERSION) || inside(x, y, POINTER)
        || rowRects.some((rect) => inside(x, y, rect)) || inside(x, y, CREST)
        || SLIDERS.some((rect) => inside(x, y, rect))) continue;
      compared += 1;
      if (!same(port, x, y, a0, x, y)) differing.push([x, y]);
    }
  }
  result.compared = compared;
  result.differing = differing.length;
  result.first_differing = differing.slice(0, 20);
  assert.equal(differing.length, 0, `${differing.length} pixels differ from A0`);

  let crestDiffering = 0;
  for (let y = 0; y < 19; y += 1) {
    for (let x = 0; x < 26; x += 1) {
      // The crest's key pixels show the row's background, which differs
      // between the rows; compare the pixels the crest covers.
      if (same(port, 85 + x, 83 + y, empty, 85 + x, 83 + y)) continue;
      if (!same(port, 85 + x, 83 + y, a0, 85 + x, 209 + y)) crestDiffering += 1;
    }
  }
  result.crest_differing = crestDiffering;
  // Its first DIB pixel, the bottom-left, is the key (COMMON 10055).
  assert.ok(same(port, 85, 101, empty, 85, 101), "the crest's key pixel shows the row's background");
  assert.equal(crestDiffering, 0, "the Empire save's crest differs from A0's (COMMON 10055)");

  result.sliders = SLIDERS.map(([x0, y0, w, h]) => {
    const columns = [];
    for (let x = x0; x < x0 + w; x += 1) {
      for (let y = y0; y < y0 + h; y += 1) {
        if (!same(port, x, y, a0, x, y)) { columns.push(x); break; }
      }
    }
    const runs = [];
    for (const x of columns) {
      if (runs.length && x === runs.at(-1)[1] + 1) runs.at(-1)[1] = x;
      else runs.push([x, x]);
    }
    return runs;
  });
  for (const runs of result.sliders) {
    assert.ok(runs.length <= 2 && runs.every(([a, b]) => b - a < 12), `a slider differs beyond its thumbs: ${JSON.stringify(runs)}`);
  }

  result.texts = TEXTS.map(([name, rect, ink, centred]) => {
    const a = inkBox(a0, rect, ink);
    const p = inkBox(port, rect, ink);
    return { name, a0: a, port: p, centred };
  });
  for (const { name, a0: a, port: p, centred } of result.texts) {
    assert.ok(a && p, `${name}: no ink`);
    const near = (u, v) => Math.abs(u - v) <= 1;
    if (centred) assert.ok(near((a[0] + a[2]) / 2, (p[0] + p[2]) / 2), `${name}: centre ${p} vs ${a}`);
    else assert.ok(near(a[0], p[0]), `${name}: left ${p} vs ${a}`);
    assert.ok(near(a[1], p[1]) && near(a[3], p[3]), `${name}: top/bottom ${p} vs ${a}`);
    const ratio = (p[2] - p[0] + 1) / (a[2] - a[0] + 1);
    assert.ok(ratio > 0.85 && ratio < 1.15, `${name}: width ratio ${ratio.toFixed(3)}`);
  }

  // Drag the music thumb to slider x 100: floor(100 * 100 / 201) = 49
  // per cent, thumb centre floor(201 * 49 / 100) = 98, left 93.
  await page.mouse.move(392 + 150, 157);
  await page.mouse.down();
  await page.mouse.move(392 + 100, 157, { steps: 4 });
  await page.mouse.up();
  await page.waitForTimeout(300);
  await page.mouse.move(330, 250);
  await page.waitForTimeout(300);
  const dragged = await shot("dragged");
  const thumb = [];
  for (let x = 392; x < 593; x += 1) {
    let differs = false;
    for (let y = 134; y < 181; y += 1) if (!same(dragged, x, y, a0, x, y)) differs = true;
    if (differs) thumb.push(x);
  }
  result.dragged_columns = [thumb[0], thumb.at(-1)];
  // The thumb's own columns, 392 + 93 .. 392 + 103, are the first run.
  assert.equal(thumb[0], 392 + 93, `the dragged thumb starts at ${thumb[0]}`);
  assert.ok(thumb.filter((x) => x <= 392 + 103).length === 11, "the dragged thumb is 11 columns wide");

  // The settings outlive the page (rebellion_data::settings, the original's
  // registry values): reload, reopen, and the thumb is where it was left.
  result.stored_settings = await page.evaluate(() => localStorage.getItem("rebellion_settings_v1"));
  assert.ok(result.stored_settings?.includes('"music_volume":0.49'), `stored ${result.stored_settings}`);
  await page.reload();
  await page.waitForFunction(() => (window.__openRebellionInterfaceSystemFinders || []).length > 0,
    null, { timeout: 30_000 });
  await page.locator("#glcanvas").focus();
  await page.waitForTimeout(500);
  await page.keyboard.press("F1");
  await page.waitForTimeout(1000);
  await page.mouse.move(330, 250);
  await page.waitForTimeout(300);
  const reloaded = await shot("reloaded");
  let reloadedDiffering = 0;
  for (let y = 134; y < 181; y += 1) {
    for (let x = 392; x < 593; x += 1) if (!same(reloaded, x, y, dragged, x, y)) reloadedDiffering += 1;
  }
  result.reloaded_slider_differing = reloadedDiffering;
  assert.equal(reloadedDiffering, 0, "the music slider did not keep its position across a reload");
  assert.deepEqual(errors, []);
  result.status = "pass";
} catch (error) {
  result.error = String(error.stack || error);
} finally {
  await browser.close();
  server.close();
}
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: result.status, compared: result.compared,
  differing: result.differing, crest_differing: result.crest_differing, sliders: result.sliders,
  dragged_columns: result.dragged_columns, reloaded_slider_differing: result.reloaded_slider_differing,
  error: result.error }, null, 2));
if (result.status !== "pass") process.exitCode = 1;
