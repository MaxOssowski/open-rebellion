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
// `--only=alliance/icons` runs the named cases alone.
const only = process.argv.filter((arg) => arg.startsWith("--only=")).map((arg) => arg.slice(7));
const expectedRequests = ["/", "/data/runtime.orpk", "/gl.js", "/open-rebellion-test.wasm"];
// Fixture codes are the Scenario index plus one (interface_test_fixture.rs).
const QUADRANTS = 50;
const factions = [
  { name: "alliance", byte: 1, side: 1, other: 2 },
  { name: "empire", byte: 2, side: 2, other: 1 },
];
// FUN_0045ca80(kind, side, 0): the first bitmap of each kind for sides
// 0 (and 3), 1 and 2; the fleet and mission kinds have none for side 0.
const art = {
  system: [10787, 10771, 10779],
  defenses: [10789, 10773, 10781],
  fleets: [null, 10775, 10783],
  missions: [null, 10777, 10785],
};
// What the fixture stocks (interface_test_fixture.rs,
// place_quadrant_contents): the side each checked icon shows, or null where
// nothing at the system lights it.
function expectedSides(faction) {
  return {
    primary: {
      system: faction.side,
      defenses: faction.side,
      fleets: faction.side,
      // FUN_004a1f60: the other side's mission wins.
      missions: faction.other,
    },
    second: {
      system: null,
      // FUN_0045ce80: the system's side, held by the other side.
      defenses: faction.other,
      fleets: null,
      missions: faction.side,
    },
  };
}
const runId = `${new Date().toISOString().replace(/[:.]/g, "-")}-${process.pid}`;
const runDir = path.join(root, ".artifacts/interface-parity", `sector-quadrants-${runId}`);

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function sourceDirectory() {
  const candidates = [
    process.env.REBELLION_STRATEGY_BMP_DIR,
    path.join(root, "data/base/ui/strategy-dll/BMP"),
  ].filter(Boolean);
  const directory = candidates.find((candidate) => fs.existsSync(path.join(candidate, "10771.bmp")));
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

function decodeIndexedBmp(bytes) {
  assert.equal(bytes.toString("ascii", 0, 2), "BM", "source resource is not a BMP");
  const dataOffset = bytes.readUInt32LE(10);
  const headerSize = bytes.readUInt32LE(14);
  const width = bytes.readInt32LE(18);
  const signedHeight = bytes.readInt32LE(22);
  const height = Math.abs(signedHeight);
  assert.deepEqual([bytes.readUInt16LE(28), bytes.readUInt32LE(30)], [8, 0],
    "source resource is not an uncompressed 8-bit BMP");
  const paletteOffset = 14 + headerSize;
  const stride = (width + 3) & ~3;
  assert.ok(bytes.length >= dataOffset + stride * height);
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
  return png;
}

const used = new Set();

function resource(source, id) {
  used.add(id);
  return decodeIndexedBmp(fs.readFileSync(path.join(source, `${id}.bmp`)));
}

// The palette-blue matte every quadrant icon carries (bmp_cache.rs,
// uses_blue_screen_transparency, 10771..=10790).
function blueKey(data, offset) {
  return data[offset] < 32 && data[offset + 1] < 32 && data[offset + 2] > 192;
}

// The icon's opaque pixels against the screenshot at the overlay's top-left
// (port: paint_native draws the bitmap at its own size there). The matte
// shows whatever lies behind, so it stays out of the check.
function compareIcon(screenshot, rect, icon, label, directory) {
  const [left, top] = rect;
  const diff = new PNG({ width: icon.width, height: icon.height });
  const actual = new PNG({ width: icon.width, height: icon.height });
  let checked = 0;
  let different = 0;
  for (let y = 0; y < icon.height; y += 1) {
    for (let x = 0; x < icon.width; x += 1) {
      const from = (y * icon.width + x) * 4;
      const at = ((Math.round(top) + y) * screenshot.width + Math.round(left) + x) * 4;
      actual.data.set(screenshot.data.subarray(at, at + 4), from);
      if (blueKey(icon.data, from)) {
        diff.data.set([0, 0, 96, 255], from);
        continue;
      }
      checked += 1;
      const matches = screenshot.data[at] === icon.data[from]
        && screenshot.data[at + 1] === icon.data[from + 1]
        && screenshot.data[at + 2] === icon.data[from + 2];
      if (!matches) different += 1;
      diff.data.set(matches ? [0, 0, 0, 255] : [255, 0, 80, 255], from);
    }
  }
  fs.writeFileSync(path.join(directory, `${label}-actual.png`), PNG.sync.write(actual));
  fs.writeFileSync(path.join(directory, `${label}-expected.png`), PNG.sync.write(icon));
  fs.writeFileSync(path.join(directory, `${label}-diff.png`), PNG.sync.write(diff));
  return { label, pixels_checked: checked, different_pixels: different };
}

async function frames(page, count = 1) {
  for (let index = 0; index < count; index += 1) {
    await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  }
}

// Two clicks inside egui's double-click time.
async function doubleClick(page, point) {
  await page.mouse.move(point.x, point.y);
  await frames(page);
  for (let index = 0; index < 2; index += 1) {
    await page.mouse.down();
    await frames(page);
    await page.mouse.up();
    await frames(page);
  }
}

function observations(page) {
  return page.evaluate(() => window.__openRebellionInterfaceQuadrants || []);
}

async function latest(page) {
  const list = await observations(page);
  assert.ok(list.length > 0, "the fixture reported no quadrant observation");
  return list[list.length - 1];
}

// Wait until the latest observation meets `predicate(observation, argument)`.
async function until(page, description, predicate, argument = null, timeout = 10_000) {
  try {
    await page.waitForFunction(
      ([source, value]) => {
        const last = (window.__openRebellionInterfaceQuadrants || []).at(-1);
        return last && new Function("o", "a", `return (${source})(o, a);`)(last, value);
      },
      [predicate.toString(), argument],
      { timeout, polling: 100 },
    );
  } catch (error) {
    throw new Error(`${description}: ${JSON.stringify((await observations(page)).slice(-3))}`, { cause: error });
  }
  return latest(page);
}

async function shot(page, directory, label) {
  const bytes = await page.screenshot({ animations: "disabled" });
  fs.writeFileSync(path.join(directory, `${label}.png`), bytes);
  return { label, sha256: sha256(bytes), bytes };
}

// The first draw requests each newly shown bitmap from the WASM texture
// cache; let the upload reach a later paint, then require two equal frames.
async function stableScreen(page, directory, label) {
  await page.mouse.move(2, 2);
  await page.waitForTimeout(150);
  await frames(page);
  const first = await page.screenshot({ animations: "disabled" });
  await frames(page);
  const second = await page.screenshot({ animations: "disabled" });
  fs.writeFileSync(path.join(directory, `${label}-screen.png`), second);
  assert.equal(sha256(first), sha256(second), `the screen did not settle (${label})`);
  const screenshot = PNG.sync.read(second);
  assert.deepEqual([screenshot.width, screenshot.height], [640, 480]);
  return screenshot;
}

function quadrant(setup, system, name) {
  const dat = system === "primary" ? setup.primary_dat_id : setup.second_dat_id;
  const found = setup.quadrants.find((entry) => entry.system_dat_id === dat && entry.quadrant === name);
  assert.ok(found, `the setup reports ${system}/${name}`);
  return found.rect;
}

const cases = [
  {
    name: "icons",
    // FUN_0045d140 draws each enabled overlay with FUN_0045ca80's art for
    // its side, and nothing for a disabled one.
    async run(page, faction, setup, directory, source) {
      const screenshot = await stableScreen(page, directory, "icons");
      const compares = [];
      const absent = [];
      for (const [system, sides] of Object.entries(expectedSides(faction))) {
        for (const [name, side] of Object.entries(sides)) {
          const rect = quadrant(setup, system, name);
          assert.deepEqual(rect.slice(2), [28, 19], `${system}/${name} is 28 by 19`);
          if (side !== null) {
            const id = art[name][side];
            compares.push({ id, ...compareIcon(screenshot, rect, resource(source, id), `${system}-${name}`, directory) });
            continue;
          }
          // Hidden: no art of the kind matches there.
          for (const id of art[name].filter((value) => value !== null)) {
            const check = compareIcon(screenshot, rect, resource(source, id), `${system}-${name}-not-${id}`, directory);
            absent.push({ id, ...check });
            assert.ok(check.different_pixels > check.pixels_checked / 2,
              `${system}/${name} shows ${id}: ${check.different_pixels} of ${check.pixels_checked} differ`);
          }
        }
      }
      for (const check of compares) {
        assert.equal(check.different_pixels, 0,
          `${check.label} (${check.id}): ${check.different_pixels} of ${check.pixels_checked} pixels differ`);
      }
      return { compares, absent };
    },
  },
  {
    name: "open-system",
    // FUN_004593e0 case 0x203 finds the shown overlay under the point;
    // FUN_0045aac0 maps kind 4 to the System window (type 9). The point lies
    // on the top-left icon, left of the planet's picture.
    async run(page, faction, setup) {
      const [left, top] = quadrant(setup, "primary", "system");
      const at = { x: Math.round(left) + 4, y: Math.round(top) + 9 };
      await doubleClick(page, at);
      const opened = await until(page, "the system icon opens the System window",
        (o, dat) => o.system_windows.some(([system]) => system === dat), setup.primary_dat_id);
      assert.equal(opened.system_windows.length, 1, JSON.stringify(opened));
      return { at, opened };
    },
  },
];

async function inspect(server, source, faction, testCase, executable) {
  const directory = path.join(runDir, `${faction.name}-${testCase.name}`);
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

    const fixtureCode = QUADRANTS | (faction.byte << 8);
    await page.goto(`${serverOrigin}/?fixture-code=${fixtureCode}`, { waitUntil: "load", timeout: 30_000 });
    await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status, null, { timeout: 30_000 });
    const ready = await page.evaluate(() => window.__openRebellionInterfaceReady);
    assert.equal(ready.status, "ready", JSON.stringify(ready));
    assert.equal(ready.code, fixtureCode);
    await page.waitForFunction(() => window.__openRebellionInterfaceQuadrantSetup
      && (window.__openRebellionInterfaceQuadrants || []).length > 0, null, { timeout: 10_000 });
    const setup = await page.evaluate(() => window.__openRebellionInterfaceQuadrantSetup);
    assert.equal(setup.code, fixtureCode);
    assert.equal(setup.scale, 1, "the gate compares at the original's scale");
    assert.equal(setup.quadrants.length, 8, JSON.stringify(setup));
    const start = await latest(page);
    assert.deepEqual(start.system_windows, [], JSON.stringify(start));
    await page.evaluate(() => document.fonts.ready);
    const { bytes: _ready, ...before } = await shot(page, directory, "ready");

    const checks = await testCase.run(page, faction, setup, directory, source);
    await page.mouse.move(2, 2);
    await frames(page);
    const { bytes: _final, ...after } = await shot(page, directory, "final");
    assert.deepEqual(requests.map(({ url }) => url).sort(), [...expectedRequests].sort());
    assert.ok(requests.every(({ status }) => status === 200), "startup has non-200 requests");
    assert.deepEqual(errors, [], `browser diagnostics: ${errors.join("; ")}`);
    result = {
      status: "pass",
      faction: faction.name,
      case: testCase.name,
      fixture_code: fixtureCode,
      ready,
      setup,
      checks,
      observations: await observations(page),
      screenshots: [before, after],
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
      case: testCase.name,
      error: String(error.stack || error),
      observations: page ? await observations(page).catch(() => []) : [],
      requests,
      errors,
      console: consoleLines,
      launch_attempts: launchAttempts,
      cleanup: "pending",
    };
    if (page) await shot(page, directory, "failure").catch(() => {});
  } finally {
    if (page) await page.close().catch((error) => errors.push(`page-close:${error}`));
    if (context) await context.close().catch((error) => errors.push(`context-close:${error}`));
    if (browser) await browser.close().catch((error) => errors.push(`browser-close:${error}`));
    result.cleanup = errors.some((error) => error.includes("-close:")) ? "failed" : "closed";
    if (result.cleanup === "failed") result.status = "fail";
  }
  return result;
}

function sourceIdentity(source) {
  const ids = [...used].sort((left, right) => left - right);
  const aggregate = createHash("sha256");
  for (const id of ids) {
    aggregate.update(`${id}\0`);
    aggregate.update(fs.readFileSync(path.join(source, `${id}.bmp`)));
  }
  return { dll: "STRATEGY.DLL", resource_ids: ids, aggregate_sha256: aggregate.digest("hex") };
}

async function main() {
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
  const source = sourceDirectory();
  const executable = browserExecutable();
  const server = await startServer();
  let results;
  try {
    results = [];
    for (const faction of factions) {
      for (const testCase of cases) {
        if (only.length && !only.includes(`${faction.name}/${testCase.name}`)) continue;
        results.push(await inspect(server, source, faction, testCase, executable));
      }
    }
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
  const passed = results.every((result) => result.status === "pass");
  const summary = {
    schema_version: 1,
    family: "sector-quadrants",
    scope: "test-only sector window quadrant icons (FUN_00459e30): each shown icon's art against STRATEGY.DLL, hidden icons absent, and the system icon opening the System window, on both sides",
    status: passed ? "pass" : "fail",
    browser_version: browserManifest.version,
    browser_executable: executable,
    launch_arguments: browserManifest.launch_arguments,
    profile: "one-new-muted-process-per-faction-and-case",
    muted: browserManifest.launch_arguments.includes("--mute-audio"),
    viewport: { width: 640, height: 480, device_scale_factor: 1 },
    source: sourceIdentity(source),
    results,
    wasm_sha256: sha256(fs.readFileSync(path.join(site, "open-rebellion-test.wasm"))),
    runtime_pack_sha256: sha256(fs.readFileSync(path.join(site, "data/runtime.orpk"))),
  };
  fs.writeFileSync(path.join(runDir, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
  if (!passed) {
    throw new Error(JSON.stringify(results.filter((result) => result.status !== "pass")
      .map(({ faction, case: name, error, observations: list }) => ({ faction, case: name, error, observations: (list || []).slice(-4) })), null, 2));
  }
  process.stdout.write(`${JSON.stringify({ run_dir: runDir, status: summary.status, cases: results.map((r) => `${r.faction}/${r.case}`), wasm_sha256: summary.wasm_sha256 }, null, 2)}\n`);
}

await main();
