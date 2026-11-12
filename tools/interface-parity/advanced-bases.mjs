#!/usr/bin/env node

// Advanced bases gate: on each side, the System window's Shipyards,
// Training Facilities and Construction Yards pages with a built Advanced
// Shipyard, Advanced Training Facility and Advanced Construction Yard of
// the player's as their first cells. Each cell's 67 by 35 picture is
// compared with its STRATEGY bitmap, every opaque pixel exact.
//
// FUN_00458fe0 picks the picture: FUN_00539f50's record 4, 5 and 6 (the
// advanced bases in MANFACSD.DAT, families 0x28, 0x29 and 0x2a) give the
// bases 0x2332, 0x233a and 0x2342 (9010, 9018, 9026); a built object
// (`+0x50` bit 2 set, bit 4 clear) takes the base whatever its side.
//
// A0: no capture of a built advanced base exists. The 2026-11-12 scan of
// .artifacts/a0-vm found 9018's pixels only in personnel-states/35, which
// shows 9020 (under construction; 9018's opaque pixels lie within it). A
// capture added to A0_PAGES is compared with the same pixels and must not
// also match base + 1..3; a listed file that is missing fails the gate.

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
// Fixture code 108 is Scenario::AdvancedBases (107) plus one.
const scenarioCode = 108;
const factions = [{ name: "alliance", byte: 1 }, { name: "empire", byte: 2 }];
// The fixture's System window stands at (225, 76); the tabs are 32 apart,
// Shipyards left of Training Facilities, Construction Yards right of it.
// A page's first picture is keyed at the window's (9, 79) (FUN_004568a0:
// the list at (8, 77), the picture centred in a 69 by 40 cell).
const windowOrigin = [225, 76];
const firstCell = [9, 79];
const pages = [
  { label: "advanced-shipyard", tabOffset: -32, picture: 9010 },
  { label: "advanced-training-facility", tabOffset: 0, picture: 9018 },
  { label: "advanced-construction-yard", tabOffset: 32, picture: 9026 },
];
// label -> { file (under the repository root), window: [x, y] }, for side 1.
const A0_PAGES = {};
const runId = `${new Date().toISOString().replace(/[:.]/g, "-")}-${process.pid}`;
const runDir = path.join(root, ".artifacts/interface-parity", `advanced-bases-${runId}`);

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function sourceDirectory() {
  const candidates = [
    process.env.REBELLION_STRATEGY_BMP_DIR,
    path.join(root, "data/base/ui/strategy-dll/BMP"),
  ].filter(Boolean);
  const directory = candidates.find((candidate) => fs.existsSync(path.join(candidate, "9010.bmp")));
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

// An 8-bit STRATEGY bitmap as RGB, with its key: the colour of the first
// byte of its bits (FUN_005fd170).
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
  const png = new PNG({ width, height });
  for (let y = 0; y < height; y += 1) {
    const sourceY = signedHeight > 0 ? height - 1 - y : y;
    for (let x = 0; x < width; x += 1) {
      const palette = paletteOffset + bytes[dataOffset + sourceY * stride + x] * 4;
      const destination = (y * width + x) * 4;
      png.data[destination] = bytes[palette + 2];
      png.data[destination + 1] = bytes[palette + 1];
      png.data[destination + 2] = bytes[palette];
      png.data[destination + 3] = 255;
    }
  }
  const key = paletteOffset + bytes[dataOffset] * 4;
  png.key = bytes[key + 2] << 16 | bytes[key + 1] << 8 | bytes[key];
  return png;
}

// The bitmap's opaque pixels against `shot` with the bitmap at (x, y).
function opaqueDiffering(shot, bitmap, [x0, y0]) {
  let checked = 0;
  let differing = 0;
  for (let y = 0; y < bitmap.height; y += 1) {
    for (let x = 0; x < bitmap.width; x += 1) {
      const from = (y * bitmap.width + x) * 4;
      if ((bitmap.data[from] << 16 | bitmap.data[from + 1] << 8 | bitmap.data[from + 2]) === bitmap.key) continue;
      const at = ((y0 + y) * shot.width + x0 + x) * 4;
      checked += 1;
      if (shot.data[at] !== bitmap.data[from] || shot.data[at + 1] !== bitmap.data[from + 1]
        || shot.data[at + 2] !== bitmap.data[from + 2]) differing += 1;
    }
  }
  return { checked, differing };
}

async function frames(page) {
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

async function click(page, point, button = "left") {
  // egui resolves a click across frames: hover, press, then release.
  await page.mouse.move(point.x, point.y);
  await frames(page);
  await page.mouse.down({ button });
  await frames(page);
  await page.mouse.up({ button });
  await frames(page);
}

async function inspect(server, source, faction, executable) {
  const directory = path.join(runDir, faction.name);
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
    page.on("requestfailed", (request) => errors.push(`requestfailed:${request.url()}:${request.failure()?.errorText}`));
    page.on("pageerror", (error) => errors.push(`pageerror:${error.stack || error.message}`));
    page.on("console", (message) => {
      consoleLines.push({ type: message.type(), text: message.text() });
      if (message.type() === "error" || /\[bmp_cache\] asset unavailable/i.test(message.text())) {
        errors.push(`console:${message.type()}:${message.text()}`);
      }
    });

    const fixtureCode = scenarioCode | (faction.byte << 8);
    await page.goto(`${serverOrigin}/?fixture-code=${fixtureCode}`, { waitUntil: "load", timeout: 30_000 });
    await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status, null, { timeout: 30_000 });
    const ready = await page.evaluate(() => window.__openRebellionInterfaceReady);
    assert.equal(ready.status, "ready", JSON.stringify(ready));
    await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.training_tab,
      null, { timeout: 10_000 });
    const report = await page.evaluate(() => window.__openRebellionInterfaceProduction);
    assert.equal(report.code, fixtureCode, JSON.stringify(report));
    const [tx, ty, tw, th] = report.training_tab;
    const checks = [];
    for (const { label, tabOffset, picture } of pages) {
      await click(page, { x: tx + tw / 2 + tabOffset, y: ty + th / 2 });
      await page.mouse.move(2, 2);
      await frames(page);
      await page.waitForTimeout(300);
      await frames(page);
      const bytes = await page.screenshot({ animations: "disabled" });
      fs.writeFileSync(path.join(directory, `${label}-page.png`), bytes);
      const bitmap = decodeIndexedBmp(fs.readFileSync(path.join(source, `${picture}.bmp`)), 67, 35);
      const at = [windowOrigin[0] + firstCell[0], windowOrigin[1] + firstCell[1]];
      const shot = PNG.sync.read(bytes);
      const entry = { label, picture, at: firstCell, screenshot_sha256: sha256(bytes),
        ...opaqueDiffering(shot, bitmap, at) };
      // The built picture's opaque pixels lie within its under-construction
      // ones (A0 personnel-states/35 matches both 9018 and 9020), so the
      // cell must also differ from base + 1, + 2 and + 3.
      entry.siblings = [1, 2, 3].map((offset) => ({ picture: picture + offset,
        ...opaqueDiffering(shot, decodeIndexedBmp(fs.readFileSync(path.join(source, `${picture + offset}.bmp`)), 67, 35), at) }));
      const a0 = A0_PAGES[label];
      if (a0 && faction.byte === 1) {
        const file = path.join(root, a0.file);
        assert.ok(fs.existsSync(file), `A0 capture ${a0.file} is missing`);
        const a0Shot = PNG.sync.read(fs.readFileSync(file));
        const a0At = [a0.window[0] + firstCell[0], a0.window[1] + firstCell[1]];
        entry.a0 = { file: a0.file, ...opaqueDiffering(a0Shot, bitmap, a0At),
          siblings_matching: [1, 2, 3].filter((offset) => opaqueDiffering(a0Shot,
            decodeIndexedBmp(fs.readFileSync(path.join(source, `${picture + offset}.bmp`)), 67, 35), a0At).differing === 0) };
        assert.deepEqual(entry.a0.siblings_matching, [], `${a0.file} also matches an unbuilt picture`);
      }
      checks.push(entry);
    }
    assert.ok(checks.every(({ checked, differing, a0, siblings }) => checked > 400 && differing === 0
      && (a0?.differing ?? 0) === 0 && siblings.every((sibling) => sibling.differing > 0)), JSON.stringify(checks));
    assert.deepEqual(errors, [], JSON.stringify(errors));
    result = {
      status: "pass",
      faction: faction.name,
      fixture_code: fixtureCode,
      checks,
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
    for (const faction of factions) results.push(await inspect(server, source, faction, executable));
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
  const passed = results.every((result) => result.status === "pass");
  const ids = pages.map(({ picture }) => picture);
  const aggregate = createHash("sha256");
  for (const id of ids) {
    aggregate.update(`${id}\0`);
    aggregate.update(fs.readFileSync(path.join(source, `${id}.bmp`)));
  }
  const summary = {
    schema_version: 1,
    family: "advanced-bases",
    scope: "test-only System window facility pages (FUN_00458fe0) with a built Advanced Shipyard, Advanced Training Facility and Advanced Construction Yard first: each picture's opaque pixels compared exactly at the window's (9, 79)",
    status: passed ? "pass" : "fail",
    browser_version: browserManifest.version,
    browser_executable: executable,
    viewport: { width: 640, height: 480, device_scale_factor: 1 },
    source: { dll: "STRATEGY.DLL", resource_ids: ids, aggregate_sha256: aggregate.digest("hex") },
    a0_pages: A0_PAGES,
    factions: results,
    wasm_sha256: sha256(fs.readFileSync(path.join(site, "open-rebellion-test.wasm"))),
    runtime_pack_sha256: sha256(fs.readFileSync(path.join(site, "data/runtime.orpk"))),
  };
  fs.writeFileSync(path.join(runDir, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
  if (!passed) throw new Error(JSON.stringify(results.filter((result) => result.status !== "pass"), null, 2));
  process.stdout.write(`${JSON.stringify({ run_dir: runDir, status: summary.status,
    factions: results.map(({ faction, status, checks }) => ({ faction, status,
      checks: checks.map(({ label, picture, checked, differing }) => ({ label, picture, checked, differing })) })) }, null, 2)}\n`);
}

await main();
