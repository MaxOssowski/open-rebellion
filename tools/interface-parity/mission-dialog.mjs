#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
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
const expectedRequests = ["/", "/data/runtime.orpk", "/gl.js", "/open-rebellion-test.wasm"];
// FUN_0046a750 builds a 259 by 355 window; the port centers it on whole pixels
// in the galaxy view (hyp: FUN_00606980). At 640 by 480 that is:
const dimensions = { width: 259, height: 355 };
const factions = [
  { name: "alliance", byte: 1, origin: { x: 168, y: 38 }, title: 10801, tabs: [[11103, 11104], [11107, 11108]], headers: [11121, 11122] },
  { name: "empire", byte: 2, origin: { x: 231, y: 40 }, title: 10802, tabs: [[11105, 11106], [11109, 11110]], headers: [11123, 11124] },
];
// Fixture codes are the Scenario index plus one (interface_test_fixture.rs).
const scenarios = [
  { name: "mission", code: 43 },
  { name: "agents", code: 44 },
];
// FUN_0046a9c0 controls: (x, y, control width, control height, normal bitmap).
const bottomButtons = [
  { x: 33, y: 320, width: 64, height: 33, id: 10592, native: [66, 33] },
  { x: 102, y: 320, width: 64, height: 33, id: 10594, native: [66, 33] },
  { x: 170, y: 320, width: 64, height: 33, id: 10596, native: [66, 33] },
];
// Dynamic content, inspected from the saved captures rather than composed:
// text (title, "Target", names), the kind item, the target art and the lists.
const masks = {
  common: [
    { x: 4, y: 2, width: 100, height: 17 },
  ],
  mission: [
    { x: 35, y: 62, width: 200, height: 113 },
    { x: 35, y: 193, width: 60, height: 16 },
    { x: 51, y: 211, width: 165, height: 79 },
    { x: 37, y: 292, width: 185, height: 18 },
  ],
  agents: [
    { x: 8, y: 93, width: 108, height: 213 },
    { x: 136, y: 93, width: 108, height: 213 },
  ],
};
const runId = `${new Date().toISOString().replace(/[:.]/g, "-")}-${process.pid}`;
const runDir = path.join(root, ".artifacts/interface-parity", `mission-dialog-${runId}`);

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function sourceDirectory() {
  const candidates = [
    process.env.REBELLION_STRATEGY_BMP_DIR,
    path.join(root, "data/base/ui/strategy-dll/BMP"),
  ].filter(Boolean);
  const directory = candidates.find((candidate) => fs.existsSync(path.join(candidate, "11100.bmp")));
  if (!directory) {
    throw new Error("owned STRATEGY.DLL BMP extraction is unavailable; set REBELLION_STRATEGY_BMP_DIR");
  }
  return directory;
}

function browserExecutable() {
  const candidates = [
    ...(process.env.OPEN_REBELLION_CHROME_FOR_TESTING
      ? [process.env.OPEN_REBELLION_CHROME_FOR_TESTING]
      : []),
    ...browserManifest.executable_candidates,
  ];
  const executable = candidates.find((candidate) => fs.existsSync(candidate));
  if (!executable) throw new Error(`pinned Chrome for Testing ${browserManifest.version} is missing`);
  const version = spawnSync(executable, ["--version"], { encoding: "utf8" });
  if (version.status !== 0 || !version.stdout.includes(browserManifest.version)) {
    throw new Error(`Chrome for Testing version mismatch: ${version.stdout || version.stderr}`);
  }
  return executable;
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

function decodeIndexedBmp(bytes, expectedWidth, expectedHeight) {
  assert.equal(bytes.toString("ascii", 0, 2), "BM", "source resource is not a BMP");
  const dataOffset = bytes.readUInt32LE(10);
  const headerSize = bytes.readUInt32LE(14);
  const width = bytes.readInt32LE(18);
  const signedHeight = bytes.readInt32LE(22);
  const height = Math.abs(signedHeight);
  assert.deepEqual([width, height, bytes.readUInt16LE(28), bytes.readUInt32LE(30)],
    [expectedWidth, expectedHeight, 8, 0], "source resource has unexpected dimensions or encoding");
  const paletteOffset = 14 + headerSize;
  const stride = (width + 3) & ~3;
  assert.ok(bytes.length >= dataOffset + stride * height);
  const png = new PNG({ width, height });
  for (let y = 0; y < height; y += 1) {
    const sourceY = signedHeight > 0 ? height - 1 - y : y;
    for (let x = 0; x < width; x += 1) {
      const paletteIndex = bytes[dataOffset + sourceY * stride + x];
      const palette = paletteOffset + paletteIndex * 4;
      const destination = (y * width + x) * 4;
      png.data[destination] = bytes[palette + 2];
      png.data[destination + 1] = bytes[palette + 1];
      png.data[destination + 2] = bytes[palette];
      png.data[destination + 3] = 255;
    }
  }
  return png;
}

function resource(source, id, width, height) {
  return decodeIndexedBmp(fs.readFileSync(path.join(source, `${id}.bmp`)), width, height);
}

function blitClipped(destination, source, x, y, width = source.width, height = source.height) {
  // FUN_005fc140 blits at the bitmap's own size; the control clips the rest.
  for (let row = 0; row < Math.min(height, source.height); row += 1) {
    if (y + row >= destination.height) break;
    const columns = Math.min(width, source.width, destination.width - x);
    const start = row * source.width * 4;
    destination.data.set(source.data.subarray(start, start + columns * 4), ((y + row) * destination.width + x) * 4);
  }
}

function resourceIds() {
  const ids = new Set([11100, 11101, 10108, 10606, 11117, 11119]);
  for (const button of bottomButtons) ids.add(button.id);
  for (const faction of factions) {
    ids.add(faction.title);
    for (const pair of faction.tabs) for (const id of pair) ids.add(id);
    for (const id of faction.headers) ids.add(id);
  }
  return [...ids].sort((left, right) => left - right);
}

function sourceIdentity(source) {
  const sortedIds = resourceIds();
  const aggregate = createHash("sha256");
  for (const resourceId of sortedIds) {
    aggregate.update(`${resourceId}\0`);
    aggregate.update(fs.readFileSync(path.join(source, `${resourceId}.bmp`)));
  }
  return {
    dll: "STRATEGY.DLL",
    resource_count: sortedIds.length,
    resource_ids: sortedIds,
    aggregate_sha256: aggregate.digest("hex"),
  };
}

function composeExpected(source, faction, page) {
  const expected = resource(source, page === "mission" ? 11100 : 11101, dimensions.width, dimensions.height);
  const title = resource(source, faction.title, 240, 17);
  blitClipped(expected, title, 2, 2);
  blitClipped(expected, title, 32, 2);
  blitClipped(expected, resource(source, 10108, 14, 14), 242, 3);
  const [missionTab, agentsTab] = faction.tabs;
  blitClipped(expected, resource(source, missionTab[page === "mission" ? 1 : 0], 116, 33), 7, 20);
  blitClipped(expected, resource(source, agentsTab[page === "agents" ? 1 : 0], 116, 33), 137, 20);
  if (page === "mission") {
    blitClipped(expected, resource(source, 10606, 65, 18), 101, 174);
  } else {
    blitClipped(expected, resource(source, faction.headers[0], 108, 27), 8, 65);
    blitClipped(expected, resource(source, faction.headers[1], 108, 27), 136, 65);
    blitClipped(expected, resource(source, 11117, 16, 16), 120, 136);
    blitClipped(expected, resource(source, 11119, 16, 16), 120, 221);
  }
  for (const button of bottomButtons) {
    blitClipped(expected, resource(source, button.id, ...button.native), button.x, button.y, button.width, button.height);
  }
  return expected;
}

function masked(page, x, y) {
  return [...masks.common, ...masks[page]].some((mask) => x >= mask.x && x < mask.x + mask.width
    && y >= mask.y && y < mask.y + mask.height);
}

function cropDialog(screenshotBytes, origin) {
  const screenshot = PNG.sync.read(screenshotBytes);
  assert.deepEqual([screenshot.width, screenshot.height], [640, 480]);
  const dialog = new PNG({ width: dimensions.width, height: dimensions.height });
  for (let y = 0; y < dimensions.height; y += 1) {
    const start = ((origin.y + y) * screenshot.width + origin.x) * 4;
    dialog.data.set(screenshot.data.subarray(start, start + dimensions.width * 4), y * dimensions.width * 4);
  }
  return dialog;
}

function compare(actual, expected, page, label, directory) {
  let differentPixels = 0;
  let checked = 0;
  const diff = new PNG({ width: expected.width, height: expected.height });
  for (let y = 0; y < expected.height; y += 1) {
    for (let x = 0; x < expected.width; x += 1) {
      const offset = (y * expected.width + x) * 4;
      if (masked(page, x, y)) {
        diff.data.set([0, 0, 96, 255], offset);
        continue;
      }
      checked += 1;
      const matches = actual.data[offset] === expected.data[offset]
        && actual.data[offset + 1] === expected.data[offset + 1]
        && actual.data[offset + 2] === expected.data[offset + 2];
      if (!matches) differentPixels += 1;
      diff.data.set(matches ? [0, 0, 0, 255] : [255, 0, 80, 255], offset);
    }
  }
  fs.writeFileSync(path.join(directory, `${label}-actual.png`), PNG.sync.write(actual));
  fs.writeFileSync(path.join(directory, `${label}-expected.png`), PNG.sync.write(expected));
  fs.writeFileSync(path.join(directory, `${label}-diff.png`), PNG.sync.write(diff));
  return { label, page, pixels_checked: checked, different_pixels: differentPixels };
}

async function frames(page) {
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

async function stableDialog(page, origin, directory, label) {
  // The first draw requests each newly shown bitmap from the WASM texture
  // cache; let the upload reach a later paint before hashing two frames.
  await page.waitForTimeout(150);
  await frames(page);
  const first = await page.screenshot({ animations: "disabled" });
  await frames(page);
  const second = await page.screenshot({ animations: "disabled" });
  fs.writeFileSync(path.join(directory, `${label}-screen.png`), second);
  const firstDialog = cropDialog(first, origin);
  const secondDialog = cropDialog(second, origin);
  assert.equal(sha256(PNG.sync.write(firstDialog)), sha256(PNG.sync.write(secondDialog)),
    `mission dialog did not stabilize (${label})`);
  return secondDialog;
}

async function inspect(server, source, faction, scenario, executable) {
  const directory = path.join(runDir, `${faction.name}-${scenario.name}`);
  fs.mkdirSync(directory, { recursive: true });
  const requests = [];
  const errors = [];
  const consoleLines = [];
  const launchAttempts = [];
  let browser;
  let context;
  let page;
  let result;
  try {
    browser = await launchBrowser(chromium, {
      executablePath: executable,
      headless: true,
      args: browserManifest.launch_arguments,
      timeout: 30_000,
    }, launchAttempts);
    context = await browser.newContext({
      viewport: { width: 640, height: 480 },
      deviceScaleFactor: 1,
      locale: "en-US",
      timezoneId: "America/New_York",
      colorScheme: "dark",
      reducedMotion: "reduce",
      serviceWorkers: "block",
    });
    page = await context.newPage();
    const serverOrigin = `http://127.0.0.1:${server.address().port}`;
    page.on("response", (response) => {
      if (new URL(response.url()).origin === serverOrigin) {
        requests.push({ url: new URL(response.url()).pathname, status: response.status() });
      }
    });
    page.on("requestfailed", (request) => {
      errors.push(`requestfailed:${request.url()}:${request.failure()?.errorText}`);
    });
    page.on("pageerror", (error) => errors.push(`pageerror:${error.stack || error.message}`));
    page.on("console", (message) => {
      consoleLines.push({ type: message.type(), text: message.text() });
      if (message.type() === "error" || /\[bmp_cache\] asset unavailable/i.test(message.text())) {
        errors.push(`console:${message.type()}:${message.text()}`);
      }
    });

    const fixtureCode = scenario.code | (faction.byte << 8);
    await page.goto(`${serverOrigin}/?fixture-code=${fixtureCode}`, { waitUntil: "load", timeout: 30_000 });
    await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status, null, { timeout: 30_000 });
    const ready = await page.evaluate(() => window.__openRebellionInterfaceReady);
    assert.equal(ready.status, "ready", JSON.stringify(ready));
    assert.equal(ready.code, fixtureCode);
    await page.evaluate(() => document.fonts.ready);

    const { origin } = faction;
    const comparisons = [];
    const opened = await stableDialog(page, origin, directory, "opened");
    comparisons.push(compare(opened, composeExpected(source, faction, scenario.name), scenario.name, "opened", directory));
    if (scenario.name === "mission") {
      // A click on the Agents tab (0x98) switches pages (FUN_0046c8a0).
      // egui resolves a click across frames: hover, press, then release.
      await page.mouse.move(origin.x + 137 + 58, origin.y + 20 + 16);
      await frames(page);
      await page.mouse.down();
      await frames(page);
      await page.mouse.up();
      await frames(page);
      await page.mouse.move(2, 2);
      const switched = await stableDialog(page, origin, directory, "agents-tab-clicked");
      comparisons.push(compare(switched, composeExpected(source, faction, "agents"), "agents", "agents-tab-clicked", directory));
    }

    assert.ok(comparisons.every(({ different_pixels: differentPixels }) => differentPixels === 0),
      JSON.stringify(comparisons.filter(({ different_pixels: differentPixels }) => differentPixels !== 0)));
    assert.deepEqual(requests.map(({ url }) => url).sort(), [...expectedRequests].sort());
    assert.ok(requests.every(({ status }) => status === 200), "startup has non-200 requests");
    assert.deepEqual(errors, [], `browser diagnostics: ${errors.join("; ")}`);
    result = {
      status: "pass",
      faction: faction.name,
      scenario: scenario.name,
      fixture_code: fixtureCode,
      ready,
      comparisons,
      requests,
      errors,
      console: consoleLines,
      launch_attempts: launchAttempts,
      cleanup: "pending",
    };
  } catch (error) {
    result = {
      status: "fail",
      faction: faction.name,
      scenario: scenario.name,
      error: String(error.stack || error),
      requests,
      errors,
      console: consoleLines,
      launch_attempts: launchAttempts,
      cleanup: "pending",
    };
  } finally {
    if (page) await page.close().catch((error) => errors.push(`page-close:${error}`));
    if (context) await context.close().catch((error) => errors.push(`context-close:${error}`));
    if (browser) await browser.close().catch((error) => errors.push(`browser-close:${error}`));
    result.cleanup = errors.some((error) => error.includes("-close:")) ? "failed" : "closed";
    if (result.cleanup === "failed") result.status = "fail";
  }
  return result;
}

async function main() {
  const source = sourceDirectory();
  if (!noBuild) {
    execFileSync("bash", [path.join(root, "scripts/build-interface-test-wasm.sh")], {
      cwd: root,
      env: { ...process.env },
      stdio: "inherit",
    });
  }
  for (const required of ["index.html", "gl.js", "open-rebellion-test.wasm", "data/runtime.orpk"]) {
    assert.ok(fs.existsSync(path.join(site, required)), `fixture site is missing ${required}`);
  }
  fs.mkdirSync(runDir, { recursive: true });
  const executable = browserExecutable();
  const server = await startServer();
  let results;
  try {
    results = [];
    for (const faction of factions) {
      for (const scenario of scenarios) results.push(await inspect(server, source, faction, scenario, executable));
    }
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
  const passed = results.every((result) => result.status === "pass");
  const summary = {
    schema_version: 1,
    family: "mission-dialog",
    scope: "test-only mission dialog (FUN_0046a750) on both pages: static STRATEGY chrome compared exactly; text, kind item, target art and member lists masked and kept as captures",
    status: passed ? "pass" : "fail",
    browser_version: browserManifest.version,
    browser_executable: executable,
    launch_arguments: browserManifest.launch_arguments,
    profile: "one-new-muted-process-per-faction-and-page",
    muted: browserManifest.launch_arguments.includes("--mute-audio"),
    viewport: { width: 640, height: 480, device_scale_factor: 1 },
    dialog: dimensions,
    origins: Object.fromEntries(factions.map(({ name, origin }) => [name, origin])),
    masks,
    source: sourceIdentity(source),
    factions: results,
    wasm_sha256: sha256(fs.readFileSync(path.join(site, "open-rebellion-test.wasm"))),
    runtime_pack_sha256: sha256(fs.readFileSync(path.join(site, "data/runtime.orpk"))),
  };
  fs.writeFileSync(path.join(runDir, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
  if (!passed) throw new Error(JSON.stringify(results.filter((result) => result.status !== "pass"), null, 2));
  process.stdout.write(`${JSON.stringify({ run_dir: runDir, ...summary }, null, 2)}\n`);
}

await main();
