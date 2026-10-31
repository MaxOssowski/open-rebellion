// The main menu's states against A0 (main_menu.rs, ghidra/notes/
// main-menu-animation.md).
//
// Serves the interface-fixture site without a fixture, skips the intro and
// replays the A0 session `.artifacts/a0-vm/main-menu-states` (--a0): the
// galaxy lever three times (FUN_00603cd0: Large, Huge, then Standard), the
// game type twice (TEXTCOMM 0x1061, then 0x1060), Intermediate, Expert and
// Easy, and the Empire button held, then released off it. After each step
// it compares the regions that do not animate with the A0 frame of that
// step, pixel for pixel: the lever, the three size lights, the selected
// difficulty (its pressed art, FUN_00602d30) and the held Empire button.
// The game-type label's ink box must match A0's within the accepted text
// deviation. Releasing off the button must not start a game.
//
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node main-menu-states.mjs --a0=.artifacts/a0-vm/main-menu-states [--out=DIR]
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
assert.ok(args.a0, "--a0=DIR (the A0 main-menu-states captures) is required");
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `main-menu-states-${new Date().toISOString().replace(/[:.]/g, "-")}`));
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".orpk": "application/octet-stream", ".webm": "video/webm" };

// main_menu.rs CONTROL_RECTS.
const LEVER = [242, 271, 44, 47];
const SIZE_LIGHTS = [290, 293, 96, 21];
const EASY = [61, 41, 51, 36];
const INTERMEDIATE = [124, 40, 49, 36];
const EXPERT = [187, 41, 45, 36];
const EMPIRE = [153, 308, 62, 55];
const LABEL = [271, 370, 110, 22];
const centre = ([x, y, w, h]) => [x + Math.floor(w / 2), y + Math.floor(h / 2)];
// [A0 frame, the click before it, the regions compared exactly, label]
const STEPS = [
  ["01-menu", null, [LEVER, SIZE_LIGHTS, EASY], true],
  ["02-lever-1", centre(LEVER), [LEVER, SIZE_LIGHTS, EASY], false],
  ["03-lever-2", centre(LEVER), [LEVER, SIZE_LIGHTS, EASY], false],
  ["04-lever-3", centre(LEVER), [LEVER, SIZE_LIGHTS, EASY], false],
  ["05-type-2", [326, 348], [LEVER, SIZE_LIGHTS, EASY], true],
  ["06-type-1", [326, 348], [LEVER, SIZE_LIGHTS, EASY], true],
  ["07-intermediate", centre(INTERMEDIATE), [LEVER, SIZE_LIGHTS, INTERMEDIATE], false],
  ["08-expert", centre(EXPERT), [LEVER, SIZE_LIGHTS, EXPERT], false],
  ["09-easy", centre(EASY), [LEVER, SIZE_LIGHTS, EASY], false],
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
function differing(port, a0, [rx, ry, rw, rh]) {
  let count = 0;
  for (let y = ry; y < ry + rh; y += 1) {
    for (let x = rx; x < rx + rw; x += 1) if (rgb(port, x, y) !== rgb(a0, x, y)) count += 1;
  }
  return count;
}
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
  const consoleLog = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  page.on("console", (message) => consoleLog.push(message.text()));
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  for (const deadline = Date.now() + 90_000; ;) {
    if (await page.locator("#main-menu-semantics[data-active=true]").count() > 0) break;
    if (Date.now() > deadline) throw new Error("the main menu never became active");
    if (await page.evaluate(() => Boolean(document.querySelector("video")))) await page.keyboard.press("Escape");
    await page.waitForTimeout(500);
  }
  await page.mouse.move(2, 470);
  await page.waitForTimeout(1000);
  const shot = async (name) => {
    const bytes = await page.screenshot({ path: path.join(OUT, `${name}.png`) });
    return { png: PNG.sync.read(bytes), sha256: createHash("sha256").update(bytes).digest("hex") };
  };
  for (const [frame, click, regions, label] of STEPS) {
    if (click) {
      await page.mouse.move(...click);
      await page.waitForTimeout(150);
      await page.mouse.down();
      await page.waitForTimeout(150);
      await page.mouse.up();
      await page.waitForTimeout(300);
      await page.mouse.move(2, 470);
      await page.waitForTimeout(500);
    }
    const port = await shot(frame);
    const a0 = PNG.sync.read(await fs.readFile(path.join(args.a0, `${frame}.png`)));
    const step = { frame, sha256: port.sha256, differing: regions.map((region) => differing(port.png, a0, region)) };
    if (label) step.ink = { a0: inkBox(a0), port: inkBox(port.png) };
    result.steps.push(step);
    assert.deepEqual(step.differing, regions.map(() => 0), `${frame}: ${JSON.stringify(step)}`);
    if (label) {
      const [a, p] = [step.ink.a0, step.ink.port];
      assert.ok(Math.abs(a[0] - p[0]) <= 1 && Math.abs(a[1] - p[1]) <= 1 && Math.abs(a[3] - p[3]) <= 1,
        `${frame}: label ink ${p} vs A0 ${a}`);
      const ratio = (p[2] - p[0] + 1) / (a[2] - a[0] + 1);
      assert.ok(ratio > 0.85 && ratio < 1.15, `${frame}: label width ratio ${ratio.toFixed(3)}`);
    }
  }
  // The Empire button held shows its pressed art; released off it, nothing
  // starts (FUN_00602d30, the button's capture).
  await page.mouse.move(...centre(EMPIRE));
  await page.waitForTimeout(150);
  await page.mouse.down();
  await page.waitForTimeout(400);
  const held = await shot("10-empire-held");
  const a0Held = PNG.sync.read(await fs.readFile(path.join(args.a0, "10-empire-held.png")));
  // A0's pointer covers the button's centre: leave out its 24 by 32.
  const [ex, ey] = centre(EMPIRE);
  let heldDiffering = 0;
  for (let y = EMPIRE[1]; y < EMPIRE[1] + EMPIRE[3]; y += 1) {
    for (let x = EMPIRE[0]; x < EMPIRE[0] + EMPIRE[2]; x += 1) {
      if (x >= ex - 2 && x < ex + 22 && y >= ey - 2 && y < ey + 30) continue;
      if (rgb(held.png, x, y) !== rgb(a0Held, x, y)) heldDiffering += 1;
    }
  }
  result.empire_held_differing = heldDiffering;
  assert.equal(heldDiffering, 0, `the held Empire button differs from A0 in ${heldDiffering} pixels`);
  await page.mouse.move(2, 470, { steps: 4 });
  await page.waitForTimeout(150);
  await page.mouse.up();
  await page.waitForTimeout(1500);
  assert.ok(await page.locator("#main-menu-semantics[data-active=true]").count() > 0,
    "releasing off the Empire button left the main menu");
  result.errors = errors;
  assert.deepEqual(errors, []);
  result.status = "pass";
} catch (error) {
  result.error = String(error.stack || error);
} finally {
  await browser.close();
  server.close();
}
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: result.status, steps: result.steps.map(({ frame, differing, ink }) => ({ frame, differing, ink })),
  empire_held_differing: result.empire_held_differing, error: result.error }, null, 2));
if (result.status !== "pass") process.exitCode = 1;
