// The sector windows' placement against A0 (sector_window.rs,
// FUN_00429ce0, FUN_004593e0).
//
// Replays the A0 session `.artifacts/a0-vm/sector-windows` (--a0) in the
// Alliance Popular Support fixture (code 5): a system in Sesswenna opens
// its sector on the left half, one in Sumitra opens on the right half,
// Sesswenna's side switch moves it to the right half and closes Sumitra
// there, and a system in Churba then opens on the freed left half. After
// each step it compares, pixel for pixel with the A0 frame, each sector
// window's title-bar buttons (side switch and close; there is no minimize
// box, FUN_004593e0) and the window's outer frame (its fixed place), and
// the sector name's ink box within the accepted text deviation. The
// windows' contents show each game's state and are left out.
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
// [A0 frame, the click before it, the halves holding a window]
const STEPS = [
  ["01-first", [252, 162], ["left"]],
  ["02-second", [470, 240], ["left", "right"]],
  ["03-flip", [270, 44], ["right"]],
  ["04-left", [115, 215], ["left", "right"]],
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
    await page.mouse.move(...click, { steps: 4 });
    await page.waitForTimeout(150);
    await page.mouse.down();
    await page.waitForTimeout(120);
    await page.mouse.up();
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
      };
    }
    result.steps.push(step);
    for (const [half, { expected, differing: counts, ink }] of Object.entries(step.halves)) {
      if (!expected) {
        assert.equal(ink.port, null, `${frame} ${half}: the port shows a window A0 does not`);
        continue;
      }
      assert.deepEqual(counts, counts.map(() => 0), `${frame} ${half}: ${JSON.stringify(step.halves[half])}`);
      const [a, p] = [ink.a0, ink.port];
      assert.ok(a && p, `${frame} ${half}: no title ink`);
      assert.ok(Math.abs((a[0] + a[2]) / 2 - (p[0] + p[2]) / 2) <= 1.5 && Math.abs(a[1] - p[1]) <= 1
        && Math.abs(a[3] - p[3]) <= 1, `${frame} ${half}: title ink ${p} vs A0 ${a}`);
    }
  }
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
