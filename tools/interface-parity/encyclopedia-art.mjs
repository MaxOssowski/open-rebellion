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
const fixtureCode = 39 | (1 << 8);
const sourceName = "EDATA.042";
const expectedRequests = ["/", "/data/runtime.orpk", "/gl.js", "/open-rebellion-test.wasm"];
const runId = `${new Date().toISOString().replace(/[:.]/g, "-")}-${process.pid}`;
const runDir = path.join(root, ".artifacts/interface-parity", `encyclopedia-art-${runId}`);

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function sourceDirectory() {
  const candidates = [
    process.env.REBELLION_EDATA_DIR,
    process.env.REBELLION_GAME_DIR && path.join(process.env.REBELLION_GAME_DIR, "EData"),
    path.resolve(root, "../star-wars-rebellion/EData"),
  ].filter(Boolean);
  const directory = candidates.find((candidate) => fs.existsSync(path.join(candidate, sourceName)));
  if (!directory) {
    throw new Error(`owned ${sourceName} is unavailable; set REBELLION_EDATA_DIR or REBELLION_GAME_DIR`);
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
  if (!executable) {
    throw new Error(`pinned Chrome for Testing ${browserManifest.version} is missing`);
  }
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

function decodeIndexedBmp(bytes) {
  assert.equal(bytes.toString("ascii", 0, 2), "BM", "source artwork is not a BMP");
  const dataOffset = bytes.readUInt32LE(10);
  const headerSize = bytes.readUInt32LE(14);
  const width = bytes.readInt32LE(18);
  const signedHeight = bytes.readInt32LE(22);
  const height = Math.abs(signedHeight);
  assert.deepEqual([width, height, bytes.readUInt16LE(28), bytes.readUInt32LE(30)],
    [400, 200, 8, 0], "source artwork is not the required 400x200x8 bitmap");
  assert.ok(headerSize >= 40);
  const paletteOffset = 14 + headerSize;
  const stride = (width + 3) & ~3;
  assert.ok(bytes.length >= dataOffset + stride * height);
  const png = new PNG({ width, height });
  for (let y = 0; y < height; y++) {
    const sourceY = signedHeight > 0 ? height - 1 - y : y;
    for (let x = 0; x < width; x++) {
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

function compareArtwork(screenshotBytes, sourceBytes) {
  const screenshot = PNG.sync.read(screenshotBytes);
  const source = decodeIndexedBmp(sourceBytes);
  assert.deepEqual([screenshot.width, screenshot.height], [640, 480]);
  const actual = new PNG({ width: source.width, height: source.height });
  let differentPixels = 0;
  const diff = new PNG({ width: source.width, height: source.height });
  for (let y = 0; y < source.height; y++) {
    for (let x = 0; x < source.width; x++) {
      const sourceOffset = (y * source.width + x) * 4;
      const screenshotOffset = ((120 + y) * screenshot.width + 120 + x) * 4;
      const actualColor = screenshot.data.subarray(screenshotOffset, screenshotOffset + 4);
      actual.data.set(actualColor, sourceOffset);
      const matches = actualColor[0] === source.data[sourceOffset]
        && actualColor[1] === source.data[sourceOffset + 1]
        && actualColor[2] === source.data[sourceOffset + 2];
      if (!matches) {
        differentPixels += 1;
        diff.data.set([255, 0, 80, 255], sourceOffset);
      } else {
        diff.data.set([0, 0, 0, 255], sourceOffset);
      }
    }
  }
  fs.writeFileSync(path.join(runDir, "actual-artwork.png"), PNG.sync.write(actual));
  fs.writeFileSync(path.join(runDir, "source-artwork.png"), PNG.sync.write(source));
  fs.writeFileSync(path.join(runDir, "diff-artwork.png"), PNG.sync.write(diff));
  return {
    status: differentPixels === 0 ? "source-bitmap-exact" : "mismatch",
    source_dimensions: [source.width, source.height],
    pixels_checked: source.width * source.height,
    different_pixels: differentPixels,
  };
}

async function main() {
  const edataDirectory = sourceDirectory();
  if (!noBuild) {
    execFileSync("bash", [path.join(root, "scripts/build-interface-test-wasm.sh")], {
      cwd: root,
      env: { ...process.env, REBELLION_EDATA_DIR: edataDirectory },
      stdio: "inherit",
    });
  }
  for (const required of ["index.html", "gl.js", "open-rebellion-test.wasm", "data/runtime.orpk"]) {
    assert.ok(fs.existsSync(path.join(site, required)), `fixture site is missing ${required}`);
  }

  fs.mkdirSync(runDir, { recursive: true });
  const sourceBytes = fs.readFileSync(path.join(edataDirectory, sourceName));
  const requests = [];
  const errors = [];
  const consoleLines = [];
  const launchAttempts = [];
  const server = await startServer();
  let browser;
  let context;
  let page;
  let result;
  try {
    browser = await launchBrowser(chromium, {
      executablePath: browserExecutable(),
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
    const origin = `http://127.0.0.1:${server.address().port}`;
    page.on("response", (response) => {
      if (new URL(response.url()).origin === origin) {
        requests.push({ url: new URL(response.url()).pathname, status: response.status() });
      }
    });
    page.on("requestfailed", (request) => {
      errors.push(`requestfailed:${request.url()}:${request.failure()?.errorText}`);
    });
    page.on("pageerror", (error) => errors.push(`pageerror:${error.stack || error.message}`));
    page.on("console", (message) => {
      consoleLines.push({ type: message.type(), text: message.text() });
      if (message.type() === "error" || /\[encyclopedia\] original artwork unavailable/i.test(message.text())) {
        errors.push(`console:${message.type()}:${message.text()}`);
      }
    });

    await page.goto(`${origin}/?fixture-code=${fixtureCode}`, {
      waitUntil: "load",
      timeout: 30_000,
    });
    await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status, null, {
      timeout: 30_000,
    });
    const ready = await page.evaluate(() => window.__openRebellionInterfaceReady);
    assert.equal(ready.status, "ready", JSON.stringify(ready));
    assert.equal(ready.code, fixtureCode);
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(250);
    const first = await page.screenshot({ animations: "disabled" });
    await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    const second = await page.screenshot({
      path: path.join(runDir, "actual.png"),
      animations: "disabled",
    });
    assert.equal(sha256(first), sha256(second), "fixture framebuffer did not stabilize");
    const comparison = compareArtwork(second, sourceBytes);
    assert.equal(comparison.different_pixels, 0,
      `${comparison.different_pixels}/${comparison.pixels_checked} artwork pixels differ`);
    assert.deepEqual(requests.map(({ url }) => url).sort(), [...expectedRequests].sort());
    assert.ok(requests.every(({ status }) => status === 200), "startup has non-200 requests");
    assert.deepEqual(errors, [], `browser diagnostics: ${errors.join("; ")}`);

    result = {
      schema_version: 1,
      family: "encyclopedia-artwork-transport",
      scope: "test-only transport/cache/decode/render proof; not original encyclopedia UI acceptance",
      status: "pass",
      fixture_code: fixtureCode,
      browser_version: browser.version(),
      browser_executable: browserExecutable(),
      launch_arguments: browserManifest.launch_arguments,
      profile: "new-process-and-temporary-profile",
      muted: browserManifest.launch_arguments.includes("--mute-audio"),
      viewport: { width: 640, height: 480, device_scale_factor: 1 },
      ready,
      comparison,
      source: { filename: sourceName, sha256: sha256(sourceBytes) },
      screenshot_sha256: sha256(second),
      wasm_sha256: sha256(fs.readFileSync(path.join(site, "open-rebellion-test.wasm"))),
      runtime_pack_sha256: sha256(fs.readFileSync(path.join(site, "data/runtime.orpk"))),
      requests,
      errors,
      console: consoleLines,
      launch_attempts: launchAttempts,
      cleanup: "pending",
    };
  } catch (error) {
    result = {
      schema_version: 1,
      family: "encyclopedia-artwork-transport",
      status: "fail",
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
    await new Promise((resolve) => server.close(resolve));
    result.cleanup = errors.some((error) => error.includes("-close:")) ? "failed" : "closed";
    if (result.cleanup === "failed") result.status = "fail";
    fs.writeFileSync(path.join(runDir, "result.json"), `${JSON.stringify(result, null, 2)}\n`);
  }
  if (result.status !== "pass") throw new Error(result.error || JSON.stringify(result));
  process.stdout.write(`${JSON.stringify({ run_dir: runDir, ...result }, null, 2)}\n`);
}

await main();
