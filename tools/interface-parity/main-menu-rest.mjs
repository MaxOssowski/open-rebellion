// The main menu at rest against A0 (agent_docs/main-menu-parity.md,
// crates/rebellion-render/src/main_menu.rs).
//
// Serves the interface-fixture site without a fixture, skips the intro with
// Escape, rests the pointer off every control and captures four frames. It
// then compares them with the A0 main-menu captures given by --a0 (the
// `2[23]?-mm-*-base.png` frames of .artifacts/a0-vm/tooltip, taken right
// after the pointer arrived, before any tip). Pixels that change between
// A0 frames or between port frames are the animated buttons (FUN_00603870
// loop on one 66 ms clock, FUN_00405560 -> FUN_00603b90) and are left out;
// so are the port's own music toggle and Fleet Registry chip (DEV-UI-001,
// DEV-UI-011) and the button rectangles that animate. What remains must
// match A0 pixel for pixel, apart from the game-type label, whose glyphs
// are drawn smoothed (the accepted text deviation): its ink box must
// match A0's.
//
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node main-menu-rest.mjs --a0=.artifacts/a0-vm/tooltip [--out=DIR]
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
assert.ok(args.a0, "--a0=DIR (the A0 main-menu captures) is required");
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `main-menu-rest-${new Date().toISOString().replace(/[:.]/g, "-")}`));
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".orpk": "application/octet-stream", ".webm": "video/webm" };
// The animated buttons' rectangles (main_menu.rs CONTROL_RECTS) and the
// port's extension chips at the top right.
const ANIMATED = [[61, 41, 51, 36], [124, 40, 49, 36], [187, 41, 45, 36], [153, 308, 62, 55],
  [437, 307, 62, 55], [411, 232, 40, 37], [67, 381, 51, 61], [536, 393, 63, 64]];
const EXTENSIONS = [[520, 0, 120, 40]];
const LABEL = [271, 370, 110, 22];

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
const pixel = (png, x, y) => {
  const i = (y * png.width + x) * 4;
  return (png.data[i] << 16) | (png.data[i + 1] << 8) | png.data[i + 2];
};
// The label's green ink box (0x00ff00 text).
function inkBox(png) {
  let box = null;
  for (let y = LABEL[1]; y < LABEL[1] + LABEL[3]; y += 1) {
    for (let x = LABEL[0]; x < LABEL[0] + LABEL[2]; x += 1) {
      const i = (y * png.width + x) * 4;
      if (png.data[i + 1] > 150 && png.data[i] < 120) {
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)]
          : [x, y, x, y];
      }
    }
  }
  return box;
}

await fs.mkdir(OUT, { recursive: true });
const a0Files = (await fs.readdir(args.a0)).filter((name) => /^2[23]\d-mm-.*-base\.png$/.test(name)).sort();
assert.ok(a0Files.length >= 2, `need A0 main-menu base frames in ${args.a0}`);
const a0 = await Promise.all(a0Files.map(async (name) => PNG.sync.read(await fs.readFile(path.join(args.a0, name)))));

const server = await serve(SITE);
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium",
  headless: true,
  args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
let result;
try {
  const page = await (await browser.newContext({ viewport: { width: 640, height: 480 } })).newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  for (const deadline = Date.now() + 90_000; ;) {
    if (await page.locator("#main-menu-semantics[data-active=true]").count() > 0) break;
    if (Date.now() > deadline) throw new Error("the main menu never became active");
    if (await page.evaluate(() => Boolean(document.querySelector("video")))) await page.keyboard.press("Escape");
    await page.waitForTimeout(500);
  }
  await page.mouse.move(2, 470);
  await page.waitForTimeout(2000);
  const port = [];
  const shots = [];
  for (let index = 0; index < 4; index += 1) {
    const bytes = await page.screenshot({ path: path.join(OUT, `port-${index}.png`) });
    shots.push(createHash("sha256").update(bytes).digest("hex"));
    port.push(PNG.sync.read(bytes));
    await page.waitForTimeout(700);
  }
  let animated = 0;
  let compared = 0;
  const differing = [];
  for (let y = 0; y < 480; y += 1) {
    for (let x = 0; x < 640; x += 1) {
      if (ANIMATED.some((rect) => inside(x, y, rect)) || EXTENSIONS.some((rect) => inside(x, y, rect))
        || inside(x, y, LABEL)) continue;
      const first = pixel(a0[0], x, y);
      if (a0.some((png) => pixel(png, x, y) !== first)
        || port.some((png) => pixel(png, x, y) !== pixel(port[0], x, y))) {
        animated += 1;
        continue;
      }
      compared += 1;
      if (pixel(port[0], x, y) !== first) differing.push([x, y]);
    }
  }
  const a0Ink = inkBox(a0[0]);
  const portInk = inkBox(port[0]);
  result = { status: "pass", compared, animated, differing: differing.length,
    first_differing: differing.slice(0, 20), a0_ink: a0Ink, port_ink: portInk, shots, errors,
    a0_frames: a0Files };
  assert.equal(differing.length, 0, `${differing.length} still pixels differ from A0`);
  // Top and bottom exact, the width exact: the line is set at GDI's pens
  // (theme::gdi_pen_positions, Arial's advances at 11 ppem). The left and
  // right ink edges within a pixel: the glyphs are Liberation's, whose
  // side bearings round differently from Arial's hinted ones.
  assert.deepEqual([portInk[1], portInk[3]], [a0Ink[1], a0Ink[3]],
    `the game-type label's ink box ${portInk} differs from A0 ${a0Ink}`);
  assert.equal(portInk[2] - portInk[0], a0Ink[2] - a0Ink[0], `the label's width ${portInk} vs A0 ${a0Ink}`);
  assert.ok(Math.abs(portInk[0] - a0Ink[0]) <= 1, `the label's left edge ${portInk[0]} vs A0 ${a0Ink[0]}`);
  assert.deepEqual(errors, []);
} catch (error) {
  result = { ...(result ?? {}), status: "fail", error: String(error.stack || error) };
} finally {
  await browser.close();
  server.close();
}
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, ...result, first_differing: undefined, shots: undefined }, null, 2));
if (result.status !== "pass") process.exitCode = 1;
