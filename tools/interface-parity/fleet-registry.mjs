#!/usr/bin/env node

// The Fleet Registry, a port extension (crates/rebellion-render/src/fleet_registry.rs):
// the main-menu chip opens it, a mode row lights, a hovered name shows its
// lore, Escape closes it with the chip's lamp lit, and a new game started
// afterwards names its fleets canonically. The viewport is the cockpit's own
// 640x480, so every point below is a logical cockpit point.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
import { PNG } from "pngjs";
import { launchBrowser } from "./browser-launch.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "../..");
const site = path.join(root, ".artifacts/interface-parity/site");
const browserManifest = JSON.parse(fs.readFileSync(path.join(here, "browser.json"), "utf8"));
const noBuild = process.argv.includes("--no-build");
const runId = `${new Date().toISOString().replace(/[:.]/g, "-")}-${process.pid}`;
const runDir = path.join(root, ".artifacts/interface-parity", `fleet-registry-${runId}`);

// main_menu.rs FLEET_REGISTRY_RECT, and its lamp at (right - 9, top + 3).
const CHIP = { x: 573, y: 21 };
const CHIP_LAMP = { x: 582, y: 15 };
// fleet_registry.rs rows: a mode row's lamp is its first 12 pixels.
const ORIGINAL_LAMP = { x: 122, y: 139 };
const CANONICAL_ROW = { x: 320, y: 165 };
const CANONICAL_LAMP = { x: 122, y: 165 };
const TITLE = { x: 100, y: 96, width: 440, height: 24 };
const EMPIRE_LANE = { x: 160, y: 200, width: 360, height: 22 };
const ALLIANCE_LANE = { x: 160, y: 236, width: 360, height: 22 };
const LORE = { x: 116, y: 270, width: 408, height: 40 };
// The cockpit's Empire monitor (COMMON.DLL control geometry).
const EMPIRE = { x: 184, y: 335 };

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function mimeType(file) {
  if (file.endsWith(".html")) return "text/html; charset=utf-8";
  if (file.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (file.endsWith(".wasm")) return "application/wasm";
  return "application/octet-stream";
}

async function startServer() {
  const server = http.createServer((request, response) => {
    const pathname = new URL(request.url, "http://localhost").pathname;
    const relative = path.posix.normalize(decodeURIComponent(pathname)).replace(/^\/+/, "")
      || "index.html";
    const candidate = path.resolve(site, relative);
    if (!candidate.startsWith(`${site}${path.sep}`)) {
      response.writeHead(403).end();
      return;
    }
    fs.readFile(candidate, (error, bytes) => {
      if (error) {
        response.writeHead(404).end();
        return;
      }
      response.writeHead(200, {
        "content-type": mimeType(candidate),
        "content-length": bytes.length,
        "cache-control": "no-store",
      }).end(bytes);
    });
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return server;
}

async function shot(page, name) {
  const file = path.join(runDir, `${name}.png`);
  const bytes = await page.screenshot({ path: file });
  return { name, file: path.relative(root, file), sha256: sha256(bytes), png: PNG.sync.read(bytes) };
}

function pixel(png, { x, y }) {
  const index = (y * png.width + x) * 4;
  return [...png.data.subarray(index, index + 3)];
}

/** Pixels in `rect` of the readout's green (cockpit game-type readout). */
function green(png, rect) {
  let count = 0;
  for (let y = rect.y; y < rect.y + rect.height; y += 1) {
    for (let x = rect.x; x < rect.x + rect.width; x += 1) {
      const [r, g, b] = pixel(png, { x, y });
      if (g > 160 && r < 140 && b < 140) count += 1;
    }
  }
  return count;
}

/** A digest of `rect`'s pixels, to tell whether a region moved. */
function region(png, rect) {
  const hash = createHash("sha256");
  for (let y = rect.y; y < rect.y + rect.height; y += 1) {
    const start = (y * png.width + rect.x) * 4;
    hash.update(png.data.subarray(start, start + rect.width * 4));
  }
  return hash.digest("hex");
}

function lit(rgb) {
  const [r, g, b] = rgb;
  return g > 200 && r < 140 && b < 140;
}

function amber(rgb) {
  const [r, g, b] = rgb;
  return r > 200 && g > 150 && g < 220 && b < 110;
}

async function frames(page, count = 6) {
  for (let index = 0; index < count; index += 1) {
    await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(resolve)));
  }
}

async function click(page, { x, y }) {
  await page.mouse.move(x, y);
  await frames(page);
  await page.mouse.down();
  await frames(page, 2);
  await page.mouse.up();
  await frames(page);
}

async function main() {
  if (!noBuild) {
    execFileSync("bash", [path.join(root, "scripts/build-interface-test-wasm.sh")], {
      cwd: root,
      stdio: "inherit",
    });
  }
  fs.mkdirSync(runDir, { recursive: true });
  const executable = browserManifest.executable_candidates.find((candidate) => fs.existsSync(candidate));
  assert.ok(executable, `pinned Chrome for Testing ${browserManifest.version} is missing`);
  const server = await startServer();
  const consoleLines = [];
  const errors = [];
  const checks = {};
  let browser;
  try {
    browser = await launchBrowser(chromium, {
      executablePath: executable,
      headless: true,
      args: browserManifest.launch_arguments,
      timeout: 30_000,
    }, []);
    const context = await browser.newContext({ viewport: { width: 640, height: 480 }, deviceScaleFactor: 1 });
    const page = await context.newPage();
    page.on("console", (message) => consoleLines.push(message.text()));
    page.on("pageerror", (error) => errors.push(String(error)));
    await page.goto(`http://127.0.0.1:${server.address().port}/`, { waitUntil: "load", timeout: 30_000 });
    await page.waitForTimeout(12_000);
    await frames(page);

    const menu = await shot(page, "01-menu");
    checks.chip_lamp_unlit = !amber(pixel(menu.png, CHIP_LAMP));
    assert.ok(checks.chip_lamp_unlit, `the chip's lamp starts unlit: ${pixel(menu.png, CHIP_LAMP)}`);

    await click(page, CHIP);
    const original = await shot(page, "02-registry-original");
    checks.title_green_pixels = green(original.png, TITLE);
    assert.ok(checks.title_green_pixels > 100, "the registry draws its green title");
    assert.ok(lit(pixel(original.png, ORIGINAL_LAMP)), "ORIGINAL is lit by default");
    assert.ok(!lit(pixel(original.png, CANONICAL_LAMP)), "CANONICAL starts dark");

    await click(page, CANONICAL_ROW);
    const canonical = await shot(page, "03-registry-canonical");
    assert.ok(lit(pixel(canonical.png, CANONICAL_LAMP)), "CANONICAL lights");
    assert.ok(!lit(pixel(canonical.png, ORIGINAL_LAMP)), "ORIGINAL goes dark");

    // Hover the Empire row until a name holds it still and its lore shows;
    // the Alliance row keeps scrolling.
    let lore;
    for (let x = EMPIRE_LANE.x; x < EMPIRE_LANE.x + EMPIRE_LANE.width; x += 12) {
      await page.mouse.move(x, EMPIRE_LANE.y + EMPIRE_LANE.height / 2);
      await frames(page, 3);
      const before = await shot(page, "04-registry-lore");
      await page.waitForTimeout(400);
      const after = await shot(page, "04-registry-lore");
      const held = region(before.png, EMPIRE_LANE) === region(after.png, EMPIRE_LANE);
      if (held && region(after.png, LORE) !== region(canonical.png, LORE)) {
        checks.alliance_row_scrolls = region(before.png, ALLIANCE_LANE) !== region(after.png, ALLIANCE_LANE);
        lore = after;
        break;
      }
    }
    checks.lore_shown = Boolean(lore);
    assert.ok(checks.lore_shown, "hovering a name holds its row and shows its lore");
    assert.ok(checks.alliance_row_scrolls, "the row not hovered keeps scrolling");

    await page.keyboard.press("Escape");
    await frames(page);
    const closed = await shot(page, "05-menu-canonical");
    assert.ok(green(closed.png, TITLE) < 20, "Escape closes the registry");
    checks.chip_lamp_lit = amber(pixel(closed.png, CHIP_LAMP));
    assert.ok(checks.chip_lamp_lit, `the chip's lamp shows canonical names: ${pixel(closed.png, CHIP_LAMP)}`);

    await click(page, EMPIRE);
    await page.waitForTimeout(4_000);
    await frames(page);
    await shot(page, "06-campaign");
    checks.campaign_naming = consoleLines.find((line) => line.includes("[campaign] fleet_naming="));
    assert.match(checks.campaign_naming ?? "", /fleet_naming=Canonical/);
    await page.keyboard.press("F3");
    await frames(page, 10);
    await shot(page, "07-fleet-finder");
    await context.close();
  } finally {
    if (browser) await browser.close();
    await new Promise((resolve) => server.close(resolve));
  }
  assert.deepEqual(errors, [], `page errors: ${errors.join("; ")}`);
  const summary = {
    schema_version: 1,
    family: "fleet-registry",
    status: "pass",
    checks,
    wasm_sha256: sha256(fs.readFileSync(path.join(site, "open-rebellion-test.wasm"))),
    console: consoleLines,
  };
  fs.writeFileSync(path.join(runDir, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify({ run_dir: runDir, status: "pass", checks, wasm_sha256: summary.wasm_sha256 }, null, 2)}\n`);
}

await main();
