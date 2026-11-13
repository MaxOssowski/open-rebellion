#!/usr/bin/env node

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync, spawnSync } from "node:child_process";
import pixelmatch from "pixelmatch";
import { chromium } from "playwright-core";
import { PNG } from "pngjs";
import { performCommand } from "./command-events.mjs";
import { launchBrowser, summarizeBrowserAttempts } from "./browser-launch.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "../..");
const site = path.join(root, ".artifacts/interface-parity/site");
const browserManifest = JSON.parse(fs.readFileSync(path.join(here, "browser.json"), "utf8"));
const schemaVersion = 1;
const options = new Set(process.argv.slice(2));
const battle = options.has("--battle");
const catalog = JSON.parse(fs.readFileSync(path.join(
  here, battle ? "scenarios/tactical.catalog.json" : "scenarios/gid.catalog.json",
), "utf8"));
const all = options.has("--all");
const smoke = options.has("--smoke") || !all;
const scenarioFilter = process.argv.find((argument) => argument.startsWith("--scenario="))?.slice(11);
const updateGoldens = options.has("--update-goldens");
const noBuild = options.has("--no-build");
const runId = `${new Date().toISOString().replace(/[:.]/g, "-")}-${process.pid}`;
const runDir = path.join(root, ".artifacts/interface-parity", runId);
const baselineDir = path.join(here, "baselines/implementation/chrome-151-mac-arm64");
const originalDir = path.join(root, "docs/qa/2026-09-10-interface-parity-audit/reference-captures");
const acceptedOriginal = JSON.parse(fs.readFileSync(path.join(here, "baselines/accepted-original.json"), "utf8"));

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function fail(message) {
  throw new Error(message);
}

function mimeType(file) {
  if (file.endsWith(".html")) return "text/html; charset=utf-8";
  if (file.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (file.endsWith(".wasm")) return "application/wasm";
  if (file.endsWith(".orpk")) return "application/octet-stream";
  return "application/octet-stream";
}

async function startServer() {
  const server = http.createServer((request, response) => {
    const pathname = new URL(request.url, "http://localhost").pathname;
    const relative = path.posix.normalize(decodeURIComponent(pathname))
      .replace(/^\/+/, "") || "index.html";
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

function browserExecutable() {
  const candidates = [
    ...(process.env.OPEN_REBELLION_CHROME_FOR_TESTING
      ? [process.env.OPEN_REBELLION_CHROME_FOR_TESTING]
      : []),
    ...browserManifest.executable_candidates,
  ];
  const executable = candidates.find((candidate) => fs.existsSync(candidate));
  if (!executable) {
    fail(`pinned Chrome for Testing ${browserManifest.version} is missing; see README.md`);
  }
  const result = spawnSync(executable, ["--version"], { encoding: "utf8" });
  if (result.status !== 0 || !result.stdout.includes(browserManifest.version)) {
    fail(`Chrome for Testing version mismatch: ${result.stdout || result.stderr}`);
  }
  return executable;
}

function scenarioId(scenario, faction, viewport) {
  return `${battle ? "tactical" : "gid"}/${faction}/${scenario.slug}/${viewport.id}`;
}

function writeContactSheet(results) {
  const cards = results.map(({ id, status }) => {
    const safe = encodeURI(id);
    return `<figure><a href="${safe}/actual.png"><img src="${safe}/actual.png" alt="${id}"></a><figcaption>${status}: ${id}</figcaption></figure>`;
  }).join("\n");
  fs.writeFileSync(path.join(runDir, "contact-sheet.html"), `<!doctype html>
<html lang="en"><meta charset="utf-8"><title>${catalog.family} interface contact sheet</title>
<style>body{background:#141822;color:#e7ebf5;font:14px system-ui;margin:16px}
main{display:grid;grid-template-columns:repeat(auto-fit,minmax(310px,1fr));gap:14px}
figure{margin:0;background:#202638;padding:8px}img{width:100%;height:240px;object-fit:contain}
figcaption{overflow-wrap:anywhere;margin-top:7px}</style>
<h1>${catalog.family} interface contact sheet</h1><p>Previews link to original lossless PNGs.
Passing harness probes are not original-game parity acceptance.</p><main>${cards}</main></html>\n`);
}

function fixtureCode(scenario, faction) {
  if (battle) return scenario.fixture_codes[faction];
  return (scenario.index + 1) | ((faction === "alliance" ? 1 : 2) << 8);
}

// The original Encyclopedia's rail close (0xfb): Alliance (423,25,32,31),
// Empire (426,21,44,41), in the 470 x 330 window FUN_0045ddc0 ->
// FUN_00606980 centres in the galaxy view: origin (62, 50) for the
// Alliance, (125, 52) for the Empire, as A0 shows
// (encyclopedia-placement.mjs). The pointer then goes back to `restore`,
// off the galaxy map, before the command centre is compared.
async function closeOriginalEncyclopedia(page, point, faction, restore) {
  const close = faction === "alliance"
    ? point(62 + 423 + 16, 50 + 25 + 15)
    : point(125 + 426 + 22, 52 + 21 + 20);
  await page.mouse.move(close.x, close.y);
  await page.mouse.down();
  await page.evaluate(() => new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }));
  await performCommand(page, "command=0xfb destination=encyclopedia status=closed",
    () => page.mouse.up());
  await page.mouse.move(restore.x, restore.y);
}

async function measureBrowserRuntime(page) {
  return page.evaluate(async () => {
    const intervals = [];
    await new Promise((resolve) => {
      let previous = null;
      function sample(now) {
        if (previous !== null) intervals.push(now - previous);
        previous = now;
        if (intervals.length >= 60) resolve();
        else requestAnimationFrame(sample);
      }
      requestAnimationFrame(sample);
    });
    const sorted = [...intervals].sort((a, b) => a - b);
    const percentile = (ratio) => sorted[Math.min(
      sorted.length - 1,
      Math.ceil(sorted.length * ratio) - 1,
    )];
    const navigation = performance.getEntriesByType("navigation")[0];
    const wasmMemoryBytes = typeof wasm_memory !== "undefined"
      ? wasm_memory.buffer.byteLength
      : null;
    const jsHeap = performance.memory || null;
    return {
      sample: "60 paused requestAnimationFrame intervals after cold fixture load",
      frame_interval_ms: {
        median: percentile(0.5),
        p95: percentile(0.95),
        max: sorted[sorted.length - 1],
      },
      navigation_duration_ms: navigation?.duration ?? null,
      wasm_memory_bytes: wasmMemoryBytes,
      js_heap_used_bytes: jsHeap?.usedJSHeapSize ?? null,
      js_heap_total_bytes: jsHeap?.totalJSHeapSize ?? null,
    };
  });
}

function compareScreenshot(id, bytes, folder, capture = "actual") {
  const baselineId = capture === "actual" ? id : `${id}/${capture}`;
  const baseline = path.join(baselineDir, `${baselineId}.png`);
  const comparison = {
    original_reference_source: "owned-original-or-curated-reference-only",
    original_reference_root: path.relative(root, originalDir),
    implementation_baseline: fs.existsSync(baseline)
      ? path.relative(root, baseline)
      : null,
    status: "unbaselined",
    different_pixels: null,
    difference_ratio: null,
    tolerance: 0,
    masks: [],
  };
  if (!fs.existsSync(baseline)) {
    if (updateGoldens) {
      const authority = acceptedOriginal.accepted[baselineId];
      if (!authority || authority.disposition !== "accepted" || !/^[a-f0-9]{64}$/.test(authority.sha256)) {
        fail(`cannot create implementation golden for ${baselineId} without a reviewed original reference`);
      }
      const original = path.resolve(root, authority.path);
      if (!original.startsWith(`${originalDir}${path.sep}`) || !fs.existsSync(original)
        || sha256(fs.readFileSync(original)) !== authority.sha256) {
        fail(`original-reference integrity check failed for ${baselineId}`);
      }
      fs.mkdirSync(path.dirname(baseline), { recursive: true });
      fs.copyFileSync(path.join(folder, `${capture}.png`), baseline);
      comparison.implementation_baseline = path.relative(root, baseline);
      comparison.status = "new_regression_baseline_from_accepted_reference";
    }
    return comparison;
  }

  const actual = PNG.sync.read(bytes);
  const expectedBytes = fs.readFileSync(baseline);
  const expected = PNG.sync.read(expectedBytes);
  fs.writeFileSync(path.join(folder, `expected-${capture}.png`), expectedBytes);
  if (actual.width !== expected.width || actual.height !== expected.height) {
    return { ...comparison, status: "dimension_mismatch" };
  }
  const diff = new PNG({ width: actual.width, height: actual.height });
  const differentPixels = pixelmatch(
    actual.data,
    expected.data,
    diff.data,
    actual.width,
    actual.height,
    { threshold: 0, includeAA: true, diffColor: [255, 0, 80] },
  );
  fs.writeFileSync(path.join(folder, `diff-${capture}.png`), PNG.sync.write(diff));
  comparison.different_pixels = differentPixels;
  comparison.difference_ratio = differentPixels / (actual.width * actual.height);
  comparison.status = differentPixels === 0 ? "match" : "mismatch";
  if (updateGoldens && differentPixels) {
    comparison.status = "mismatch_not_updated";
  }
  return comparison;
}

async function stableFrame(page, folder) {
  const first = await page.screenshot({ path: path.join(folder, "frame-a.png"), animations: "disabled" });
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const second = await page.screenshot({ path: path.join(folder, "actual.png"), animations: "disabled" });
  return { first: sha256(first), second: sha256(second), bytes: second };
}

async function stableInteractionFrame(page, folder, name) {
  let previous = null;
  for (let attempt = 0; attempt < 8; attempt++) {
    await page.evaluate(() => new Promise((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(resolve))));
    const bytes = await page.screenshot({ animations: "disabled" });
    if (previous && sha256(previous) === sha256(bytes)) {
      fs.writeFileSync(path.join(folder, `${name}.png`), bytes);
      return bytes;
    }
    previous = bytes;
  }
  throw new Error(`tactical interaction capture did not stabilize: ${name}`);
}

async function dynamicInteractionFrame(page, folder, name) {
  await page.evaluate(() => new Promise((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(resolve))));
  return page.screenshot({
    path: path.join(folder, `${name}.png`),
    animations: "disabled",
  });
}

function decodeIndexedBmp(bytes) {
  assert.equal(bytes.toString("ascii", 0, 2), "BM", "source resource is not a BMP");
  const dataOffset = bytes.readUInt32LE(10);
  const headerSize = bytes.readUInt32LE(14);
  const width = bytes.readInt32LE(18);
  const height = bytes.readInt32LE(22);
  assert.equal(bytes.readUInt16LE(28), 8, "source resource is not indexed 8-bit BMP");
  assert.equal(bytes.readUInt32LE(30), 0, "source resource has unsupported compression");
  assert.ok(width > 0 && height > 0 && headerSize >= 40);
  const paletteOffset = 14 + headerSize;
  const stride = (width + 3) & ~3;
  assert.ok(bytes.length >= dataOffset + stride * height);
  return {
    width,
    height,
    index(x, y) {
      return bytes[dataOffset + (height - 1 - y) * stride + x];
    },
    pixel(x, y) {
      const index = bytes[dataOffset + (height - 1 - y) * stride + x];
      const entry = paletteOffset + index * 4;
      return [bytes[entry + 2], bytes[entry + 1], bytes[entry]];
    },
  };
}

function verifyGidRootFrame(viewport, screenshotBytes, faction, systemWindowOpen = false) {
  const rootX = faction === "alliance" ? 370 : 420;
  if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
    return { status: "non-native-scale", pixels_checked: 0 };
  }
  const screenshot = PNG.sync.read(screenshotBytes);
  const resources = new Map();
  for (let id = 10100; id <= 10107; id++) {
    resources.set(id, decodeIndexedBmp(fs.readFileSync(path.join(
      root, `data/base/ui/strategy-dll/BMP/${id}.bmp`,
    ))));
  }
  let pixelsChecked = 0;
  let pixelsOccluded = 0;
  if (systemWindowOpen) {
    const covered = (235 * screenshot.width + rootX) * 4;
    assert.notDeepEqual(
      Array.from(screenshot.data.subarray(covered, covered + 3)),
      resources.get(10100).pixel(0, 0),
      "system fixture did not occlude the GID root at its known overlap",
    );
  }
  const check = (sx, sy, id, bx, by) => {
    // The Manufacturing window (type 9, 226 by 304 at (225, 76)) spans x=225..450,
    // y=76..379. It is in front of the GID popup, so compare the authored frame
    // everywhere else.
    if (systemWindowOpen && sx <= 450 && sy <= 379) {
      pixelsOccluded++;
      return;
    }
    const source = resources.get(id);
    const offset = (sy * screenshot.width + sx) * 4;
    assert.deepEqual(
      Array.from(screenshot.data.subarray(offset, offset + 3)),
      source.pixel(bx, by),
      `GID root border (${sx}, ${sy}) differs from STRATEGY ${id} (${bx}, ${by})`,
    );
    pixelsChecked++;
  };
  // The root window is 160 x 160 at (370, 233) for the Alliance and
  // (420, 233) for the Empire (FUN_00422ce0 -> FUN_00451130; A0 gid/00-menu).
  // FUN_004511e0 hands FUN_00607740 the tiles in the Game Menu Window's
  // order, laid out as the A0 menus show: corners 10101, 10100, 10103,
  // 10102; 10105 and 10106 in every column of the top and bottom; 10107 and
  // 10104 on every second row of the sides. Check every painted border pixel.
  const right = rootX + 158;
  const bottom = 233 + 158;
  for (const [id, x, y] of [
    [10101, rootX, 233], [10100, right, 233],
    [10103, rootX, bottom], [10102, right, bottom],
  ]) {
    for (let by = 0; by < 2; by++) {
      for (let bx = 0; bx < 2; bx++) check(x + bx, y + by, id, bx, by);
    }
  }
  for (let x = rootX + 2; x < right; x++) {
    for (let by = 0; by < 2; by++) {
      check(x, 233 + by, 10105, 0, by);
      check(x, bottom + by, 10106, 0, by);
    }
  }
  for (let y = 235; y < bottom; y += 2) {
    for (let bx = 0; bx < 2; bx++) {
      check(rootX + bx, y, 10107, bx, 0);
      check(right + bx, y, 10104, bx, 0);
    }
  }
  return {
    status: systemWindowOpen ? "source-bitmap-visible-partial" : "source-bitmap-exact",
    source_bitmaps_checked: 8,
    pixels_checked: pixelsChecked,
    pixels_occluded: pixelsOccluded,
  };
}

// The first frame after the game is ready already holds the whole shell
// (CMD-01-C004): the side columns above the advisors and the bottom
// console right of them are the settled frame's, pixel for pixel. The
// original's first frame of a new game shows the complete cockpit under
// its Message Index (A0 empire-initial/00). The galaxy aperture, the top
// readouts and the animated advisors are left out.
function verifyFirstFrameComplete(viewport, firstBytes, stableBytes, folder) {
  if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
    return { status: "non-native-scale" };
  }
  fs.writeFileSync(path.join(folder, "first-ready.png"), firstBytes);
  const first = PNG.sync.read(firstBytes);
  const settled = PNG.sync.read(stableBytes);
  const regions = { left: [0, 40, 55, 300], right: [540, 40, 640, 390], console: [120, 395, 640, 480] };
  const differing = {};
  for (const [name, [x0, y0, x1, y1]] of Object.entries(regions)) {
    let count = 0;
    for (let y = y0; y < y1; y += 1) {
      for (let x = x0; x < x1; x += 1) {
        const i = (y * first.width + x) * 4;
        if (first.data[i] !== settled.data[i] || first.data[i + 1] !== settled.data[i + 1]
          || first.data[i + 2] !== settled.data[i + 2]) count += 1;
      }
    }
    differing[name] = count;
  }
  assert.ok(Object.values(differing).every((count) => count === 0),
    `the first ready frame lacks part of the shell: ${JSON.stringify(differing)}`);
  return { status: "complete", differing };
}

function verifyMessageIndexRail(faction, viewport, screenshotBytes) {
  if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
    return { status: "non-native-scale", source_bitmaps_checked: 0 };
  }
  const screenshot = PNG.sync.read(screenshotBytes);
  assert.equal(screenshot.width, 640);
  assert.equal(screenshot.height, 480);
  const x0 = faction === "alliance" ? 3 : 611;
  const y0 = faction === "alliance" ? 109 : 110;
  const firstResource = faction === "alliance" ? 10050 : 10030;
  let keyed = 0;
  for (let control = 0; control < 9; control++) {
    const resource = firstResource + control;
    const source = decodeIndexedBmp(fs.readFileSync(path.join(
      root, `data/base/ui/strategy-dll/BMP/${resource}.bmp`,
    )));
    assert.equal(source.width, 27);
    assert.equal(source.height, 22);
    // Each control is a CoolStrobeButton (FUN_00427270) blitted keyed on
    // its first DIB pixel (FUN_005fd170, the bottom row's first): A0
    // message-rail/easy-index-closed shows the shell under those pixels
    // and every other pixel as the bitmap's.
    const key = source.index(0, source.height - 1);
    for (let y = 0; y < source.height; y++) {
      for (let x = 0; x < source.width; x++) {
        if (source.index(x, y) === key) {
          keyed += 1;
          continue;
        }
        const actualOffset = ((y0 + control * 25 + y) * screenshot.width + x0 + x) * 4;
        assert.deepEqual(
          Array.from(screenshot.data.subarray(actualOffset, actualOffset + 3)),
          source.pixel(x, y),
          `${faction} rail ${resource} pixel (${x}, ${y}) differs from its original BMP`,
        );
      }
    }
  }
  return { status: "source-bitmap-exact", source_bitmaps_checked: 9, pixels_checked: 9 * 27 * 22 - keyed,
    pixels_keyed: keyed };
}

// The GID's system states (CMD-02 C018, C020, C022, C023, C024) against A0.
// FUN_0042cde0 blits each item's mark keyed (FUN_005fd0f0) at the 17 by 17
// cell's top left, FUN_00425d00's corner. A0's game states are not the
// fixture's, so each mark the port draws at the primary system is compared
// with the same STRATEGY mark where an A0 capture shows it at a system's
// exact corner, every non-key pixel. The table lists, for each mark and
// cockpit, the capture, the system and its corner (found by an exact keyed
// search at every system's corner; the largest mark that matches there).
const GID_STATE_SCENARIOS = new Map([
  ["known", []],
  ["uninhabited", []],
  // Idle Fleets (Alt+3, accelerator 0xbcc, display 0x21) and Active
  // Personnel (Alt+6, 0xbcf, 0x44), then Popular Support (Alt+1) again.
  ["blockade", ["3"]],
  ["mission", ["6"]],
  ["fleet", ["3"]],
]);
// The mark each display gives the primary system (FUN_0042b330 through
// FUN_00426e70; STRATEGY 10146..10149 the Alliance's, largest first, 10150..
// 10153 the Empire's, smallest first, as their art shows):
// - Popular Support (0x11): the player's support, 92% for the Alliance
//   (largest), 8% for the Empire (smallest), in the holder's colour;
// - Idle Fleets (0x21, bands 3, 2, 2, 1, 1): one fleet is the medium mark,
//   in its side's colour; the blockade's fleet is the enemy's;
// - Active Personnel (0x44, bands 1): the agent on a mission makes it the
//   player's largest.
const GID_STATE_EXPECTED = {
  alliance: { "Popular Support": 10146, "blockade:Alt+3": 10151,
    "fleet:Alt+3": 10148, "mission:Alt+6": 10146 },
  empire: { "Popular Support": 10150, "blockade:Alt+3": 10148, "fleet:Alt+3": 10151,
    "mission:Alt+6": 10153 },
};
const GID_KNOWN_CELLS = new Map();
const GID_A0_MARKS = {
  alliance: {
    // agent-menu/alliance-easy-day1 and alliance-expert-day1 (new Alliance
    // games, day 1, Popular Support) and 2026-10-28-nav/fleet-averam (the
    // "test" save). Not the DS_flip_allian captures (gid-selection/*),
    // whose sides are mixed.
    10146: ["agent-menu/alliance-easy-day1.png", "Corsin", 245, 155],
    10147: ["agent-menu/alliance-expert-day1.png", "Averam", 214, 146],
    10148: ["2026-10-28-nav/fleet-averam.png", "Umgul", 367, 272],
    10150: ["agent-menu/alliance-easy-day1.png", "Coruscant", 226, 150],
    10151: ["agent-menu/alliance-easy-day1.png", "Vagran", 191, 228],
    10152: ["agent-menu/alliance-expert-day1.png", "Xyquine", 182, 196],
    10155: ["agent-menu/alliance-easy-day1.png", "Bothawui", 352, 258],
  },
  empire: {
    // gid/d-popular-support, d-idle-fleets and d-energy (Empire, day 209).
    10146: ["gid/d-energy.png", "Commenor", 257, 201],
    10147: ["gid/d-energy.png", "Duros", 264, 219],
    10148: ["gid/d-idle-fleets.png", "Yavin", 526, 236],
    10149: ["gid/d-popular-support.png", "Commenor", 257, 201],
    10150: ["gid/d-idle-fleets.png", "Coruscant", 289, 152],
    10151: ["gid/d-idle-fleets.png", "Averam", 277, 148],
    10152: ["gid/d-popular-support.png", "Averam", 277, 148],
    10153: ["gid/d-popular-support.png", "Coruscant", 289, 152],
    10154: ["gid/d-popular-support.png", "Corellia", 270, 205],
    10155: ["gid/d-popular-support.png", "Yaga Minor", 290, 135],
    10156: ["gid/d-energy.png", "Yaga Minor", 290, 135],
    10157: ["gid/d-energy.png", "Svivren", 274, 169],
  },
};

function gidMarkPixels(id) {
  const bitmap = decodeIndexedBmp(fs.readFileSync(path.join(root, `data/base/ui/strategy-dll/BMP/${id}.bmp`)));
  const key = bitmap.pixel(0, bitmap.height - 1).join(",");
  const pixels = [];
  for (let y = 0; y < bitmap.height; y += 1) {
    for (let x = 0; x < bitmap.width; x += 1) {
      const rgb = bitmap.pixel(x, y);
      if (rgb.join(",") !== key) pixels.push([x, y, rgb]);
    }
  }
  return pixels;
}

// The largest STRATEGY mark (10146..10160) whose every non-key pixel the
// image shows at (left, top).
function gidMarkAt(png, left, top) {
  let found = null;
  for (let id = 10146; id <= 10160; id += 1) {
    if (id === 10159) continue;
    const pixels = gidMarkPixels(id);
    const exact = pixels.every(([x, y, rgb]) => {
      const i = ((top + y) * png.width + left + x) * 4;
      return png.data[i] === rgb[0] && png.data[i + 1] === rgb[1] && png.data[i + 2] === rgb[2];
    });
    if (exact && (!found || pixels.length > found.pixels)) found = { id, pixels: pixels.length };
  }
  return found?.id ?? null;
}

function compareGidMarkWithA0(faction, id, port, left, top) {
  const reference = GID_A0_MARKS[faction][id];
  assert.ok(reference, `${faction}: no A0 capture listed for GID mark ${id}`);
  const [file, system, a0Left, a0Top] = reference;
  const a0Path = path.join(root, ".artifacts/a0-vm", file);
  assert.ok(fs.existsSync(a0Path), `A0 capture ${file} is missing`);
  const a0 = PNG.sync.read(fs.readFileSync(a0Path));
  let different = 0;
  const pixels = gidMarkPixels(id);
  for (const [x, y] of pixels) {
    const i = ((a0Top + y) * a0.width + a0Left + x) * 4;
    const j = ((top + y) * port.width + left + x) * 4;
    if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[j + c])) different += 1;
  }
  assert.equal(gidMarkAt(a0, a0Left, a0Top), id, `A0 ${file} shows no ${id} at ${system}`);
  assert.equal(different, 0, `${faction}: ${different} of ${pixels.length} pixels of ${id} differ from A0 ${file} (${system})`);
  return { mark: id, a0: file, a0_system: system, a0_corner: [a0Left, a0Top], port_corner: [left, top],
    pixels_checked: pixels.length, different_pixels: different };
}

async function verifyGidStateMarksAgainstA0(page, faction, scenario, folder, ready) {
  const probes = [];
  const left = Math.round(ready.primary_screen_x - 8.5);
  const top = Math.round(ready.primary_screen_y - 8.5);
  // Off the galaxy, so no hover label covers the primary system.
  await page.mouse.move(2, 2);
  const shown = await stableInteractionFrame(page, folder, `${scenario.slug}-a0-marks`);
  const shot = PNG.sync.read(shown);
  const mark = ready.primary_gid_marks.at(-1);
  assert.equal(ready.mode, "Popular Support");
  assert.equal(mark, GID_STATE_EXPECTED[faction]["Popular Support"], JSON.stringify(ready));
  assert.equal(gidMarkAt(shot, left, top), mark, `${faction}: the primary system does not show ${mark}`);
  // The cell and a 2-pixel margin, as the known (unblockaded) case drew it.
  const cell = [];
  for (let y = -2; y < 19; y += 1) {
    for (let x = -2; x < 19; x += 1) {
      const i = ((top + y) * shot.width + left + x) * 4;
      cell.push(shot.data[i], shot.data[i + 1], shot.data[i + 2]);
    }
  }
  if (scenario.slug === "known") GID_KNOWN_CELLS.set(faction, cell);
  if (scenario.slug === "blockade") {
    // A0, the Empire game, one day (gid-displays/legend-alt-1 and
    // legend-alt-3, Alt+1 and Alt+3 3 s apart): Yavin is held by the
    // Alliance (Popular Support 10149, the Alliance's smallest) with one
    // Empire fleet alone in orbit (Idle Fleets 10151, the Empire's medium
    // mark, not the both-sides 10160). FUN_0050b8e0's rule (no battle, a
    // fleet of another side than the holder's) makes it blockaded. Its
    // Popular Support cell and margin equal, pixel for pixel, Yavin's on
    // day 209 (gid/d-popular-support), when the Alliance's own fleet
    // orbited it (d-idle-fleets 10148): the blockade draws nothing.
    const a0 = (file) => PNG.sync.read(fs.readFileSync(path.join(root, ".artifacts/a0-vm", file)));
    const blockaded = a0("gid-displays/legend-alt-1.png");
    assert.equal(gidMarkAt(a0("gid-displays/legend-alt-3.png"), 526, 236), 10151,
      "A0 gid-displays/legend-alt-3: Yavin's Idle Fleets mark");
    const unblockaded = a0("gid/d-popular-support.png");
    let a0Different = 0;
    for (let y = -2; y < 19; y += 1) {
      for (let x = -2; x < 19; x += 1) {
        const i = ((236 + y) * 640 + 526 + x) * 4;
        if ([0, 1, 2].some((c) => blockaded.data[i + c] !== unblockaded.data[i + c])) a0Different += 1;
      }
    }
    assert.equal(a0Different, 0, "A0's blockaded Yavin differs from its unblockaded cell");
    assert.equal(gidMarkAt(blockaded, 526, 236), 10149, "A0 gid-displays/legend-alt-1: Yavin's mark");
    // The port: the blockaded primary system's cell equals the known
    // case's, when that case ran first (run.mjs --all).
    const known = GID_KNOWN_CELLS.get(faction);
    const portDifferent = known
      ? known.reduce((count, value, index) => count + (value !== cell[index] ? 1 : 0), 0) : null;
    if (known) {
      assert.equal(portDifferent, 0, `${faction}: the blockaded system's cell differs from the known case's`);
    }
    probes.push({ type: "gid-blockade-cell", a0_yavin_cell_pixels: 21 * 21, a0_different: a0Different,
      port_compared_with_known: Boolean(known), port_different_channels: portDifferent });
  }
  probes.push({ type: "gid-state-mark-a0", display: ready.mode,
    ...compareGidMarkWithA0(faction, mark, shot, left, top) });
  // A frame between each key event, so the port's per-frame key state
  // sees Alt held when the digit goes down.
  const nextFrames = () => page.evaluate(() => new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }));
  const altDigit = async (digit) => {
    await page.keyboard.down("Alt");
    await nextFrames();
    await page.keyboard.down(`Digit${digit}`);
    await nextFrames();
    await page.keyboard.up(`Digit${digit}`);
    await page.keyboard.up("Alt");
  };
  for (const digit of GID_STATE_SCENARIOS.get(scenario.slug)) {
    await altDigit(digit);
    const frameBytes = await stableInteractionFrame(page, folder, `${scenario.slug}-alt-${digit}-a0-marks`);
    assert.notEqual(sha256(frameBytes), sha256(shown), `${faction}: Alt+${digit} changed no display`);
    const frame = PNG.sync.read(frameBytes);
    const id = gidMarkAt(frame, left, top);
    assert.equal(id, GID_STATE_EXPECTED[faction][`${scenario.slug}:Alt+${digit}`],
      `${faction}: Alt+${digit} shows ${id} at the primary system`);
    probes.push({ type: "gid-state-mark-a0", display: `Alt+${digit}`,
      ...compareGidMarkWithA0(faction, id, frame, left, top) });
  }
  if (GID_STATE_SCENARIOS.get(scenario.slug).length > 0) {
    await altDigit("1");
    const back = await stableInteractionFrame(page, folder, `${scenario.slug}-alt-1-restored`);
    assert.equal(sha256(back), sha256(shown), `${faction}: Alt+1 did not restore Popular Support`);
  }
  return probes;
}

async function probeGid(page, faction, scenario, viewport, folder, consoleLines, ready) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  // Command 0x132: rightmost bottom control in cockpit.rs for each faction.
  const control = faction === "alliance"
    ? { x: 446, y: 406, width: 27, height: 16 }
    : { x: 519, y: 434, width: 37, height: 25 };
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const interior = point(control.x + control.width / 2, control.y + control.height / 2);
  const outside = point(control.x - 2, control.y + control.height / 2);
  const probes = [];

  // A headquarters system's item carries flag 8 (FUN_0042b330,
  // FUN_00526cf0 families 0x20..0x21): FUN_0042cde0 blits the burst 10166
  // keyed at the 17 by 17 cell's top left, then the mark over it (A0
  // gid/hq-burst-alliance-day390: 10166 under 10146, 0 of 89 off).
  if (scenario.slug === "headquarters" && scale === 1) {
    assert.equal(ready.primary_gid_marks.length, 2, JSON.stringify(ready));
    assert.equal(ready.primary_gid_marks[0], 10166, JSON.stringify(ready));
    const shot = PNG.sync.read(await page.screenshot({ path: path.join(folder, "headquarters-burst.png"),
      animations: "disabled" }));
    const expected = new Map();
    for (const id of ready.primary_gid_marks) {
      const bitmap = decodeIndexedBmp(fs.readFileSync(path.join(root, `data/base/ui/strategy-dll/BMP/${id}.bmp`)));
      const key = bitmap.pixel(0, bitmap.height - 1).join(",");
      for (let y = 0; y < bitmap.height; y += 1) {
        for (let x = 0; x < bitmap.width; x += 1) {
          const rgb = bitmap.pixel(x, y);
          if (rgb.join(",") !== key) expected.set(`${x},${y}`, rgb);
        }
      }
    }
    const left = Math.round(ready.primary_screen_x - 8.5);
    const top = Math.round(ready.primary_screen_y - 8.5);
    let different = 0;
    for (const [at, rgb] of expected) {
      const [x, y] = at.split(",").map(Number);
      const index = ((top + y) * shot.width + left + x) * 4;
      if (shot.data[index] !== rgb[0] || shot.data[index + 1] !== rgb[1] || shot.data[index + 2] !== rgb[2]) different += 1;
    }
    probes.push({ type: "headquarters-burst", marks: ready.primary_gid_marks, left, top,
      pixels_checked: expected.size, different_pixels: different });
    assert.equal(different, 0, `${faction}: ${different} of ${expected.size} headquarters burst pixels differ`);
  }
  // Neither FUN_0042b330 nor FUN_0042cde0 reads the blockade bit (+0x88
  // bit 0x20; they test only bits 1 and 2): a blockaded system's item is
  // its display's mark alone.
  if (scenario.slug === "blockade" && scale === 1) {
    assert.equal(ready.primary_gid_marks.length, 1, JSON.stringify(ready));
    probes.push({ type: "blockade-marks", marks: ready.primary_gid_marks });
  }
  if (GID_STATE_SCENARIOS.has(scenario.slug) && scale === 1) {
    probes.push(...await verifyGidStateMarksAgainstA0(page, faction, scenario, folder, ready));
  }

  const commandLines = () => consoleLines.map(({ text }) => text)
    .filter((line) => line.includes("[interface] command="));
  await page.mouse.click(outside.x, outside.y);
  await page.waitForTimeout(70);
  assert.ok(!commandLines().some((line) => line.includes("destination=gid_menu status=opened")),
    `${faction}: strict outside-edge opened the GID menu`);
  probes.push({ type: "outside-control", x: outside.x, y: outside.y, opened: false });

  // FUN_00602d30 has no hover art: the control under the pointer keeps its
  // rest art (A0 tooltip/20..26 *-base: each hovered control shows its
  // normal bitmap).
  const restBytes = await page.screenshot({ animations: "disabled" });
  await page.mouse.move(interior.x, interior.y);
  await page.evaluate(() => new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }));
  const hoverBytes = await page.screenshot({ path: path.join(folder, "control-hover.png"), animations: "disabled" });
  {
    const [rest, hover] = [PNG.sync.read(restBytes), PNG.sync.read(hoverBytes)];
    const at = point(control.x, control.y);
    let differing = 0;
    for (let y = Math.round(at.y); y < Math.round(at.y + control.height * scale); y += 1) {
      for (let x = Math.round(at.x); x < Math.round(at.x + control.width * scale); x += 1) {
        const i = (y * rest.width + x) * 4;
        if (rest.data.readUInt32BE(i) !== hover.data.readUInt32BE(i)) differing += 1;
      }
    }
    assert.equal(differing, 0, `${faction}: the hovered control changed its art`);
    probes.push({ type: "control-hover", unchanged: true });
  }
  await page.mouse.down();
  await page.evaluate(() => new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }));
  const pressedBytes = await page.screenshot({ path: path.join(folder, "control-pressed.png"), animations: "disabled" });
  // Held, the Empire's control shows its pressed art 10025 (FUN_00602d30):
  // A0 controls-empire/eg1 (2026-11-05) matches it in every opaque pixel;
  // its one key-colour pixel, (0, 21), shows the shell (FUN_005fd170).
  if (faction === "empire") {
    probes.push({ type: "control-pressed-art",
      ...verifyStrategyBitmap(viewport, pressedBytes, 10025, control.x, control.y,
        [{ x: 0, y: 21, width: 1, height: 1 }]) });
  }
  const opened = "command=0x132 destination=gid_menu status=opened_original";
  await performCommand(page, opened, () => page.mouse.up());
  const rootCapture = await page.screenshot({ path: path.join(folder, "menu-root.png"), animations: "disabled" });
  const rootFrame = verifyGidRootFrame(viewport, rootCapture, faction, scenario.slug === "system");
  probes.push({ type: "control-click", x: interior.x, y: interior.y, opened: true,
    screenshot_sha256: sha256(rootCapture), root_frame: rootFrame });

  const closed = "command=0x132 destination=gid_menu status=closed";
  await performCommand(page, closed, () => page.mouse.click(interior.x, interior.y));
  const closedCapture = await page.screenshot({ path: path.join(folder, "control-closed.png"),
    animations: "disabled" });
  assert.equal(sha256(closedCapture), sha256(fs.readFileSync(path.join(folder, "actual.png"))),
    `${faction}: the GID control did not close its own menu cleanly`);
  probes.push({ type: "control-click-close", x: interior.x, y: interior.y, opened: false,
    screenshot_sha256: sha256(closedCapture) });
  await performCommand(page, opened, () => page.mouse.click(interior.x, interior.y));

  // The GID menu window (FUN_00422ce0 -> FUN_00451130): 160 x 160 at
  // (370, 233) for the Alliance and (420, 233) for the Empire; categories at
  // y 10 + 22 k, children at 7 + 22 k (FUN_004511e0); a submenu 240 wide,
  // placed by FUN_004ab3a0 beside the root.
  const rootX = faction === "alliance" ? 370 : 420;
  const rootY = 233;
  const submenuHeights = [55, 55, 55, 100, 142, 122];
  const submenuTop = (category) => {
    const top = 10 + 22 * category;
    return top + submenuHeights[category] < 160
      ? rootY + top - 3
      : rootY + top + 20 - submenuHeights[category] + 3;
  };
  const leafAt = (category, leaf) => point(rootX - 240 + 3 + 80, submenuTop(category) + 7 + 22 * leaf + 10);
  if (scenario.category !== null) {
    const alternate = scenario.mode === "Uprisings" ? "Popular Support" : "Uprisings";
    const alternatePoint = point(rootX + 80, rootY + 10 + 10);
    await page.mouse.move(alternatePoint.x, alternatePoint.y);
    await page.waitForTimeout(50);
    const hoveredRoot = await page.screenshot({ path: path.join(folder, "menu-root-hover.png"), animations: "disabled" });
    if (viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1
        && scenario.slug !== "system") {
      const before = PNG.sync.read(rootCapture);
      const after = PNG.sync.read(hoveredRoot);
      const pixel = (png, x, y) => Array.from(png.data.subarray((y * png.width + x) * 4, (y * png.width + x) * 4 + 3));
      // A pixel of the Loyalty row's icon (30..49): hovering lights the
      // text and arrow only.
      assert.deepEqual(pixel(after, rootX + 40, rootY + 20), pixel(before, rootX + 40, rootY + 20),
        "GID root added an unproven synthetic row hover wash");
    }
    const alternateLeaf = leafAt(0, alternate === "Uprisings" ? 1 : 0);
    await performCommand(page, `destination=gid status=selected label=${alternate}`,
      () => page.mouse.click(alternateLeaf.x, alternateLeaf.y));
    await page.waitForFunction((expected) => window.__openRebellionInterfaceSelection?.mode === expected,
      alternate, { timeout: 2_000 });
    probes.push({ type: "alternate-mode", mode: alternate });

    await performCommand(page, opened, () => page.mouse.click(interior.x, interior.y));
    const rootPoint = point(rootX + 80, rootY + 10 + 22 * scenario.category + 10);
    await page.mouse.move(rootPoint.x, rootPoint.y);
    await page.waitForTimeout(80);
    const submenuCapture = await page.screenshot({ path: path.join(folder, "menu-submenu.png"), animations: "disabled" });
    probes.push({ type: "category-hover", category: scenario.category,
      screenshot_sha256: sha256(submenuCapture) });
    assert.notEqual(sha256(rootCapture), sha256(submenuCapture),
      `${scenario.slug}: submenu did not change the root view`);
    const leafPoint = leafAt(scenario.category, scenario.leaf);
    const expected = `destination=gid status=selected label=${scenario.mode}`;
    await performCommand(page, expected, () => page.mouse.click(leafPoint.x, leafPoint.y));
    await page.waitForFunction((mode) => window.__openRebellionInterfaceSelection?.mode === mode,
      scenario.mode, { timeout: 2_000 });
    const selected = await page.evaluate(() => window.__openRebellionInterfaceSelection);
    assert.equal(selected.code, fixtureCode(scenario, faction));
    const selectedCapture = await page.screenshot({ path: path.join(folder, "menu-selected.png"), animations: "disabled" });
    probes.push({ type: "submenu-selection", mode: scenario.mode,
      screenshot_sha256: sha256(selectedCapture) });
  } else if (scenario.slug === "display-off") {
    const displayOff = point(rootX + 70, rootY + 142 + 8);
    await performCommand(page, "destination=gid status=selected label=Display Off",
      () => page.mouse.click(displayOff.x, displayOff.y));
    await page.waitForFunction(() => window.__openRebellionInterfaceSelection?.mode === "Display Off",
      null, { timeout: 2_000 });
    probes.push({ type: "root-selection", mode: "Display Off" });
  } else {
    await page.keyboard.press("Escape");
    await page.waitForTimeout(80);
    const dismissed = await page.screenshot({ path: path.join(folder, "menu-dismissed.png"), animations: "disabled" });
    assert.equal(sha256(dismissed), sha256(fs.readFileSync(path.join(folder, "actual.png"))),
      `${scenario.slug}: Escape left the original galaxy surface`);
    probes.push({ type: "menu-escape", screenshot_sha256: sha256(dismissed) });
  }

  if (scenario.slug === "popular-support") {
    // Choosing a display builds its legend under the galaxy view
    // (FUN_00426d00), so the command centre to return to is the one after
    // the selection, legend button shown.
    const commandCenter = await stableInteractionFrame(page, folder,
      "command-center-after-selection");
    const commandControls = faction === "alliance"
      ? [
          { slug: "game-options", rect: { x: 3, y: 355, width: 27, height: 41 },
            expected: "command=0x133 destination=game_options status=opened_original" },
          { slug: "encyclopedia", rect: { x: 394, y: 405, width: 27, height: 16 },
            expected: "command=0x131 destination=encyclopedia status=opened_original" },
        ]
      : [
          { slug: "game-options", rect: { x: 79, y: 192, width: 35, height: 57 },
            expected: "command=0x133 destination=game_options status=opened_original" },
          { slug: "encyclopedia", rect: { x: 465, y: 434, width: 35, height: 24 },
            expected: "command=0x131 destination=encyclopedia status=opened_original" },
        ];
    for (const command of commandControls) {
      const center = point(command.rect.x + command.rect.width / 2,
        command.rect.y + command.rect.height / 2);
      await page.mouse.move(center.x, center.y);
      const hover = await page.screenshot({ path: path.join(folder, `${command.slug}-hover.png`),
        animations: "disabled" });
      await page.mouse.down();
      await page.evaluate(() => new Promise((resolve) => {
        requestAnimationFrame(() => requestAnimationFrame(resolve));
      }));
      const pressed = await page.screenshot({
        path: path.join(folder, `${command.slug}-pressed.png`), animations: "disabled",
      });
      const observed = await performCommand(page, command.expected, () => page.mouse.up());
      const released = await page.screenshot({
        path: path.join(folder, `${command.slug}-released.png`), animations: "disabled",
      });
      assert.notEqual(sha256(pressed), sha256(hover),
        `${faction}: ${command.slug} did not show its original pressed resource`);
      if (command.slug === "game-options") {
        assert.notEqual(sha256(released), sha256(commandCenter),
          `${faction}: Game Options did not open its original surface`);
        probes.push({
          type: "game-options-command-surface",
          ...verifyCommonBitmap(viewport, released, 20002, 0, 0, [
            { x: 0, y: 0, width: 640, height: 2 },
            { x: 0, y: 478, width: 640, height: 2 },
            { x: 0, y: 2, width: 2, height: 476 },
            { x: 638, y: 2, width: 2, height: 476 },
            { x: 22, y: 26, width: 596, height: 440 },
          ]),
        });
        await page.keyboard.press("Escape");
        const returned = await stableInteractionFrame(page, folder,
          `${command.slug}-returned-to-command-center`);
        assert.equal(sha256(returned), sha256(commandCenter),
          `${faction}: Game Options did not return to the authentic command center`);
      } else {
        // FUN_00429f30 opens the original Encyclopedia; its rail's close
        // (0xfb) returns to the command centre.
        assert.notEqual(sha256(released), sha256(commandCenter),
          `${faction}: the Encyclopedia did not open its original window`);
        await closeOriginalEncyclopedia(page, point, faction, center);
        const returned = await stableInteractionFrame(page, folder,
          `${command.slug}-returned-to-command-center`);
        assert.equal(sha256(returned), sha256(commandCenter),
          `${faction}: the Encyclopedia did not return to the command center`);
      }
      probes.push({ type: "physical-command", command: command.slug, expected: command.expected,
        observed, hover_sha256: sha256(hover), pressed_sha256: sha256(pressed),
        released_sha256: sha256(released) });
    }

    for (const [key, expected] of [
      ["F1", "command=0x133 destination=game_options status=opened_original"],
      ["F7", "command=0x131 destination=encyclopedia status=opened_original"],
    ]) {
      const observed = await performCommand(page, expected, () => page.keyboard.press(key));
      probes.push({ type: "keyboard-command", key, expected, observed });
      if (key === "F7") {
        const encyclopedia = await stableInteractionFrame(page, folder, "keyboard-encyclopedia-open");
        assert.notEqual(sha256(encyclopedia), sha256(commandCenter),
          `${faction}: F7 did not open the original Encyclopedia`);
        const restore = faction === "alliance" ? point(407.5, 413) : point(482.5, 446);
        await closeOriginalEncyclopedia(page, point, faction, restore);
        const returned = await stableInteractionFrame(page, folder,
          "keyboard-encyclopedia-returned");
        assert.equal(sha256(returned), sha256(commandCenter),
          `${faction}: F7 Encyclopedia did not return to the command center`);
      }
      if (key === "F1") {
        const options = await stableInteractionFrame(page, folder, "keyboard-game-options-open");
        assert.notEqual(sha256(options), sha256(commandCenter),
          `${faction}: F1 did not open the original Game Options surface`);
        await page.keyboard.press("Escape");
        const returned = await stableInteractionFrame(page, folder,
          "keyboard-game-options-returned");
        assert.equal(sha256(returned), sha256(commandCenter),
          `${faction}: F1 Game Options did not return to the command center`);
      }
    }

    // Off every control, so no tip (500 ms, FUN_006008e0) shows over the
    // frame compared.
    await page.mouse.move(offsetX + 2 * scale, offsetY + 2 * scale);
    await page.waitForTimeout(100);
    await page.keyboard.press("E");
    await page.evaluate(() => new Promise((resolve) => {
      requestAnimationFrame(() => requestAnimationFrame(resolve));
    }));
    const legacyEncyclopedia = await page.screenshot({
      path: path.join(folder, "legacy-encyclopedia-shortcut.png"), animations: "disabled",
    });
    assert.equal(sha256(legacyEncyclopedia), sha256(commandCenter),
      `${faction}: legacy E shortcut exposed a replacement Encyclopedia`);
    probes.push({ type: "legacy-encyclopedia-shortcut", key: "E", status: "fails-closed",
      screenshot_sha256: sha256(legacyEncyclopedia) });
  }

  if (scenario.mode !== null && scenario.mode !== "Display Off") {
    const legendPoint = point(faction === "alliance" ? 55 : 113, 50);
    const compact = await page.screenshot({ path: path.join(folder, "legend-compact.png"),
      clip: { x: legendPoint.x, y: legendPoint.y, width: 47 * scale, height: 25 * scale },
      animations: "disabled" });
    probes.push({ type: "compact-legend-bitmap", x: legendPoint.x, y: legendPoint.y,
      screenshot_sha256: sha256(compact) });
  }

  if (scenario.slug === "hover") {
    const system = point(ready.probe_screen_x, ready.probe_screen_y);
    await page.mouse.move(system.x, system.y);
    await page.waitForFunction((datId) => window.__openRebellionInterfaceHover?.system_dat_id === datId,
      ready.probe_system_dat_id, { timeout: 2_000 });
    const hovered = await page.evaluate(() => window.__openRebellionInterfaceHover);
    assert.equal(hovered.system_name, ready.probe_system_name);
    const capture = await page.screenshot({ path: path.join(folder, "system-hover.png"), animations: "disabled" });
    probes.push({ type: "system-hover", dat_id: hovered.system_dat_id,
      name: hovered.system_name, screenshot_sha256: sha256(capture) });
    // The hovered sector item's label (FUN_00441520, FUN_00422ce0 WM_PAINT):
    // "Moddell" over "Sector", font 5, in the palette's yellow, from the
    // item's place. A0 gid-selection/01 (Alliance, pointer at (155, 123)):
    // its (255, 255, 0) ink spans (135, 91)..(183, 116).
    if (viewport.width === 640 && viewport.height === 480 && faction === "alliance") {
      const at = point(155, 123);
      await page.mouse.move(at.x, at.y);
      const labelled = PNG.sync.read(await stableInteractionFrame(page, folder, "sector-hover-label"));
      let box = null;
      for (let y = 70; y < 118; y += 1) {
        for (let x = 90; x < 260; x += 1) {
          const i = (y * labelled.width + x) * 4;
          if (labelled.data[i] === 255 && labelled.data[i + 1] === 255 && labelled.data[i + 2] === 0) {
            box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)]
              : [x, y, x, y];
          }
        }
      }
      const a0 = [135, 91, 183, 116];
      assert.ok(box && Math.abs(box[0] - a0[0]) <= 2 && Math.abs(box[1] - a0[1]) <= 2
        && Math.abs(box[3] - a0[3]) <= 2
        && Math.abs((box[2] - box[0]) / (a0[2] - a0[0]) - 1) < 0.15,
        `the Moddell label's ink ${JSON.stringify(box)} against A0 ${JSON.stringify(a0)}`);
      probes.push({ type: "sector-hover-label", port: box, a0 });
      await page.mouse.move(system.x, system.y);
    }
    // FUN_00422ce0 opens the sector on WM_LBUTTONUP: a held press shows
    // only the hover label, the release opens the window (A0
    // gid-selection/01, 02: "Moddell Sector" while held, Moddell's window
    // on release).
    const hoverFrame = await stableInteractionFrame(page, folder, "system-before-press");
    await page.mouse.down();
    const held = await stableInteractionFrame(page, folder, "system-held");
    assert.equal(sha256(held), sha256(hoverFrame), "the press alone changed the galaxy view");
    await page.mouse.up();
    const released = await stableInteractionFrame(page, folder, "system-released");
    assert.notEqual(sha256(released), sha256(held), "the release opened nothing");
    probes.push({ type: "system-press-release", held_sha256: sha256(held),
      released_sha256: sha256(released) });
  }

  if (["pan", "zoom"].includes(scenario.slug)) {
    const start = point(320, 170);
    const before = await page.screenshot({ animations: "disabled" });
    await page.mouse.move(start.x, start.y);
    if (scenario.slug === "pan") {
      await page.mouse.down({ button: "right" });
      await page.mouse.move(start.x + 35 * scale, start.y + 15 * scale, { steps: 5 });
      await page.mouse.up({ button: "right" });
    } else {
      await page.mouse.wheel(0, -120);
    }
    await page.waitForTimeout(100);
    const after = await page.screenshot({ path: path.join(folder, `${scenario.slug}-interaction.png`), animations: "disabled" });
    // FUN_00422ce0 has no wheel case and no right-button drag: the map
    // neither zooms nor pans, so the frame is unchanged.
    assert.equal(sha256(before), sha256(after), `${scenario.slug} moved the map`);
    probes.push({ type: scenario.slug, before_sha256: sha256(before), after_sha256: sha256(after),
      unchanged: true });
  }

  if (scenario.category !== null && scenario.mode !== null) {
    // FUN_00426d00 builds the legend at (150, 120) under the galaxy view,
    // so a chosen display shows only the legend button (A0 gid/d-*.png);
    // the button (command 0x134) shows the legend. FUN_00607740 frames it
    // grey outside, white inside; its close button (FUN_004522f0) sits at
    // width - 14 - 3, 3. Closing leaves the button (FUN_00422ce0 0x405
    // case 3), which restores it.
    const exact = viewport.width === 640 && viewport.height === 480
      && viewport.device_scale_factor === 1;
    const pixelAt = (bytes, x, y) => {
      const png = PNG.sync.read(bytes);
      const at = (y * png.width + x) * 4;
      return Array.from(png.data.subarray(at, at + 3));
    };
    const chosen = await stableInteractionFrame(page, folder, "legend-chosen");
    if (exact) {
      assert.notDeepEqual(pixelAt(chosen, 151, 121), [255, 255, 255], "choosing the display opened the legend");
    }
    const legendButton = point((faction === "alliance" ? 55 : 113) + 15, 50 + 15);
    // FUN_00452ab0's flags 3: the button swallows a click and posts its
    // command on a double click only (Class_00659e38_vt14; A0
    // 2026-11-13-gid/A2 a click opens nothing, A3 a double click opens).
    await page.mouse.click(legendButton.x, legendButton.y);
    const single = await stableInteractionFrame(page, folder, "legend-single-click");
    if (exact) {
      assert.notDeepEqual(pixelAt(single, 151, 121), [255, 255, 255], "a single click opened the legend");
    }
    await page.mouse.dblclick(legendButton.x, legendButton.y);
    const open = await stableInteractionFrame(page, folder, "legend-open");
    if (exact) {
      assert.deepEqual(pixelAt(open, 150, 120), [192, 192, 192], "legend frame corner is not grey");
      assert.deepEqual(pixelAt(open, 151, 121), [255, 255, 255], "legend frame inner line is not white");
    }
    const closeButton = point(150 + 180 - 14 - 3 + 7, 120 + 3 + 7);
    await page.mouse.click(closeButton.x, closeButton.y);
    const closed = await stableInteractionFrame(page, folder, "legend-closed");
    assert.notEqual(sha256(closed), sha256(open), `${scenario.slug}: the legend did not close`);
    if (exact) {
      assert.notDeepEqual(pixelAt(closed, 151, 121), [255, 255, 255],
        "the closed legend left its frame");
    }
    const button = point((faction === "alliance" ? 55 : 113) + 15, 50 + 15);
    await page.mouse.dblclick(button.x, button.y);
    const restored = await stableInteractionFrame(page, folder, "legend-restored");
    assert.notEqual(sha256(restored), sha256(closed), `${scenario.slug}: the legend button did nothing`);
    // The button opened the legend both times, so the frames agree.
    assert.equal(sha256(restored), sha256(open), `${scenario.slug}: the restored legend differs from the opened one`);
    if (exact) {
      assert.deepEqual(pixelAt(restored, 151, 121), [255, 255, 255], "the legend did not return");
    }
    // The window drags by its body (FUN_00606650); a point inside its frame
    // away from the texts and the close button.
    const grab = point(150 + 170, 120 + 60);
    await page.mouse.move(grab.x, grab.y);
    await page.mouse.down();
    // A press lands where the pointer is in its frame; let it land first.
    await page.evaluate(() => new Promise((resolve) => {
      requestAnimationFrame(() => requestAnimationFrame(resolve));
    }));
    await page.mouse.move(grab.x + 40 * scale, grab.y + 20 * scale, { steps: 4 });
    await page.mouse.up();
    const dragged = await stableInteractionFrame(page, folder, "legend-dragged");
    assert.notEqual(sha256(dragged), sha256(restored), `${scenario.slug}: the legend did not drag`);
    if (exact) {
      assert.deepEqual(pixelAt(dragged, 190, 140), [192, 192, 192], "the dragged frame is not at +40, +20");
      assert.deepEqual(pixelAt(dragged, 191, 141), [255, 255, 255], "the dragged frame lost its inner line");
    }
    // A click on a system closes a visible legend first (FUN_00422ce0,
    // WM_LBUTTONUP with a hovered item posts 0x405). Drag the legend to the
    // top left so it leaves the probe system free.
    // Grab 5, 5 inside the frame and drop the window in the top-left
    // corner, keeping the pointer on the page so the release arrives.
    const corner = point(190 + 5, 140 + 5);
    const target = point(5, 5);
    await page.mouse.move(corner.x, corner.y);
    await page.mouse.down();
    await page.evaluate(() => new Promise((resolve) => {
      requestAnimationFrame(() => requestAnimationFrame(resolve));
    }));
    await page.mouse.move(target.x, target.y, { steps: 4 });
    await page.mouse.up();
    const cornered = await stableInteractionFrame(page, folder, "legend-cornered");
    if (exact) {
      // The page draws no frame of its own over the canvas's edge.
      assert.deepEqual(pixelAt(cornered, 0, 0), [192, 192, 192], "the legend did not reach the corner");
      assert.deepEqual(pixelAt(cornered, 1, 1), [255, 255, 255], "the moved legend lost its inner line");
    }
    const system = point(ready.probe_screen_x, ready.probe_screen_y);
    await page.mouse.move(system.x, system.y);
    await page.waitForTimeout(80);
    await performCommand(page, "command=0x405 destination=gid_legend status=closed_by_map_click",
      () => page.mouse.click(system.x, system.y));
    const clickedAway = await stableInteractionFrame(page, folder, "legend-closed-by-map-click");
    if (exact) {
      assert.notDeepEqual(pixelAt(clickedAway, 1, 1), [255, 255, 255],
        "the map click left the legend open");
    }
    probes.push({ type: "legend-window", open_sha256: sha256(open), closed_sha256: sha256(closed),
      restored_sha256: sha256(restored), dragged_sha256: sha256(dragged),
      closed_by_map_click_sha256: sha256(clickedAway) });
  }

  return probes;
}

// A squadron button: FUN_0059c030 makes it 25x15, so its bitmap's first 15
// rows show and the backdrop 1000 fills the last two (A0 TAC-02-C012).
// Task Force 1's number on its button: A0 TAC-02-C008 (at rest, black) and
// TAC-03-C001 (selected, white) draw the "1" over rows 5..14 with its stem
// at x 72. The ink is what differs from the button's bitmap (1005, or 1006
// selected) by more than 30 in luminance, lighter when selected. The
// port's face is Liberation Sans, whose "1" has a foot that Arial's lacks
// and whose stem straddles two columns: the rows within a pixel and the
// stem (the column with the most ink) within a pixel (the accepted text
// deviation).
function verifyTaskForceOneNumber(viewport, screenshotBytes, selected) {
  if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
    return { status: "non-native-scale" };
  }
  const screenshot = PNG.sync.read(screenshotBytes);
  const button = decodeIndexedBmp(fs.readFileSync(path.join(
    root, `data/base/ui/tactical-dll/BMP/${selected ? 1006 : 1005}.bmp`,
  )));
  const luminance = ([r, g, b]) => (r * 299 + g * 587 + b * 114) / 1000;
  let box = null;
  const columns = new Map();
  for (let y = 3; y < 18; y++) {
    for (let x = 64; x < 81; x++) {
      const offset = (y * screenshot.width + x) * 4;
      const shown = luminance(Array.from(screenshot.data.subarray(offset, offset + 3)));
      const under = luminance(button.pixel(x - 60, y - 2));
      const ink = selected ? shown - under : under - shown;
      if (ink <= 30) continue;
      if (y >= 8 && y <= 12) columns.set(x, (columns.get(x) ?? 0) + ink);
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)]
        : [x, y, x, y];
    }
  }
  const stem = [...columns.entries()].sort((a, b) => b[1] - a[1])[0]?.[0];
  const a0 = { rows: [5, 14], stem: 72 };
  const ok = box && Math.abs(box[1] - a0.rows[0]) <= 1 && Math.abs(box[3] - a0.rows[1]) <= 1
    && Math.abs(stem - a0.stem) <= 1;
  assert.ok(ok, `Task Force 1's ${selected ? "white" : "black"} number ${JSON.stringify({ box, stem })} against A0 ${JSON.stringify(a0)}`);
  return { status: "a0-rows-and-stem-within-a-pixel", port: { box, stem }, a0 };
}

function verifySquadronButton(viewport, screenshotBytes, id, x0, y0) {
  const button = verifyTacticalBitmap(viewport, screenshotBytes, id, x0, y0, [],
    [{ x: 0, y: 15, width: 25, height: 2 }]);
  if (button.status === "non-native-scale") return button;
  const screenshot = PNG.sync.read(screenshotBytes);
  const backdrop = decodeIndexedBmp(fs.readFileSync(path.join(root, "data/base/ui/tactical-dll/BMP/1000.bmp")));
  for (let y = y0 + 15; y < y0 + 17; y++) {
    for (let x = x0; x < x0 + 25; x++) {
      const offset = (y * screenshot.width + x) * 4;
      assert.deepEqual(Array.from(screenshot.data.subarray(offset, offset + 3)), backdrop.pixel(x, y),
        `squadron button ${id} row ${y - y0} shows the backdrop at (${x}, ${y})`);
    }
  }
  return { ...button, backdrop_rows_checked: 50 };
}

// TACTICAL bitmaps keyed on their first DIB pixel's index (FUN_005fd170;
// bmp_cache::uses_first_pixel_key): A0 TAC-02-C012, TAC-01-C002 and
// TAC-05-C017 show the backdrop 1000 under the group bar's key pixels; the
// panel's buttons show the panel there.
const TACTICAL_FIRST_PIXEL_KEYED = new Set([1001, 1004, 1005, 1006, 1007, 1008, 1010,
  ...Array.from({ length: 12 }, (_, i) => 1012 + i), 1034, 1035, 1058,
  ...Array.from({ length: 8 }, (_, i) => 1101 + i), ...Array.from({ length: 6 }, (_, i) => 1113 + i),
  ...Array.from({ length: 10 }, (_, i) => 1133 + i), ...Array.from({ length: 10 }, (_, i) => 1149 + i),
  ...Array.from({ length: 26 }, (_, i) => 1170 + i), ...Array.from({ length: 25 }, (_, i) => 1201 + i),
  1351, 1352, 1353, 1354, ...Array.from({ length: 118 }, (_, i) => 2201 + i)]);
const TACTICAL_BACKDROP_UNDER_KEY = (id) => id >= 1001 && id <= 1020;
// The subsystem icons 1201..1225 show in the battle's palette (TACTICAL
// 5531..5557): entries 140 and 146 are (230, 0, 0) and (255, 230, 60) there,
// as A0 TAC-03-C001..C011 show (bmp_cache::battle_palette_entries).
const SUBSYSTEM_ICON_BATTLE_ENTRIES = new Map([[140, [230, 0, 0]], [146, [255, 230, 60]]]);

function verifyTacticalBitmap(
  viewport, screenshotBytes, id, x0, y0, occluders = [], ignoredRects = [], skipIndices = null,
) {
  if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
    return { status: "non-native-scale", pixels_checked: 0 };
  }
  const screenshot = PNG.sync.read(screenshotBytes);
  const source = decodeIndexedBmp(fs.readFileSync(path.join(
    root, `data/base/ui/tactical-dll/BMP/${id}.bmp`,
  )));
  const decodedOccluders = occluders.map(({ id: overlayId, x, y }) => ({
    x,
    y,
    source: decodeIndexedBmp(fs.readFileSync(path.join(
      root, `data/base/ui/tactical-dll/BMP/${overlayId}.bmp`,
    ))),
  }));
  const key = TACTICAL_FIRST_PIXEL_KEYED.has(id) ? source.index(0, source.height - 1) : null;
  const backdrop = key !== null && TACTICAL_BACKDROP_UNDER_KEY(id)
    ? decodeIndexedBmp(fs.readFileSync(path.join(root, "data/base/ui/tactical-dll/BMP/1000.bmp")))
    : null;
  let pixelsChecked = 0;
  let keyPixelsChecked = 0;
  for (let y = 0; y < source.height; y++) {
    for (let x = 0; x < source.width; x++) {
      if (ignoredRects.some((rect) => x >= rect.x && y >= rect.y
        && x < rect.x + rect.width && y < rect.y + rect.height)) continue;
      if (skipIndices && skipIndices(source.index(x, y))) continue;
      let color = source.pixel(x, y);
      if (id >= 1201 && id <= 1225 && SUBSYSTEM_ICON_BATTLE_ENTRIES.has(source.index(x, y))) {
        color = SUBSYSTEM_ICON_BATTLE_ENTRIES.get(source.index(x, y));
      }
      if (key !== null && source.index(x, y) === key) {
        if (!backdrop) continue;
        color = backdrop.pixel(x0 + x, y0 + y);
        keyPixelsChecked++;
      } else if (key === null && color[0] < 32 && color[1] < 32 && color[2] > 192) continue;
      const screenX = x0 + x;
      const screenY = y0 + y;
      const occluded = decodedOccluders.some(({ x: overlayX, y: overlayY, source: overlay }) => {
        const localX = screenX - overlayX;
        const localY = screenY - overlayY;
        if (localX < 0 || localY < 0 || localX >= overlay.width || localY >= overlay.height) {
          return false;
        }
        const overlayColor = overlay.pixel(localX, localY);
        return !(overlayColor[0] < 32 && overlayColor[1] < 32 && overlayColor[2] > 192);
      });
      if (occluded) continue;
      const actualOffset = ((y0 + y) * screenshot.width + x0 + x) * 4;
      assert.deepEqual(
        Array.from(screenshot.data.subarray(actualOffset, actualOffset + 3)), color,
        `tactical control ${id} pixel (${x}, ${y}) differs from its original BMP`,
      );
      pixelsChecked++;
    }
  }
  return { status: "source-bitmap-exact", resource_id: id, pixels_checked: pixelsChecked,
    key_pixels_on_backdrop: keyPixelsChecked };
}

function verifyGokresBitmap(viewport, screenshotBytes, id, x0, y0) {
  if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
    return { status: "non-native-scale", pixels_checked: 0 };
  }
  const screenshot = PNG.sync.read(screenshotBytes);
  const source = decodeIndexedBmp(fs.readFileSync(path.join(
    root, `data/base/ui/gokres-dll/BMP/${id}.bmp`,
  )));
  let pixelsChecked = 0;
  for (let y = 0; y < source.height; y++) {
    for (let x = 0; x < source.width; x++) {
      const color = source.pixel(x, y);
      if (color[0] < 32 && color[1] < 32 && color[2] > 192) continue;
      const actualOffset = ((y0 + y) * screenshot.width + x0 + x) * 4;
      assert.deepEqual(
        Array.from(screenshot.data.subarray(actualOffset, actualOffset + 3)), color,
        `GOKRES content ${id} pixel (${x}, ${y}) differs from its original BMP`,
      );
      pixelsChecked++;
    }
  }
  assert.ok(pixelsChecked > 0, `GOKRES content ${id} contained no opaque proof pixels`);
  return { status: "source-bitmap-exact", resource_id: id, pixels_checked: pixelsChecked };
}

// A0 TAC-01-C007 (the Empire's Battle at Yavin, day 234, Show Planet off,
// first paused frame): only the starfield and the holocube's blue edges
// fill the battle view. The port's first Imperial frame draws the same
// edges (FUN_005d4d10's cube under FUN_005d9640's opening camera): every
// blue edge pixel of either lies within two pixels of the other's, and no
// planet backdrop lights the view (A0 732 lit non-blue pixels: stars, one
// hull and the status line; a planet disc alone would be thousands).
function compareEmptySpaceWithA0(viewport, screenshotBytes, faction) {
  const a0Path = path.join(root, ".artifacts/interface-parity/a0/TAC-01-C007/original.png");
  if (faction !== "empire" || viewport.width !== 640 || viewport.height !== 480
    || viewport.device_scale_factor !== 1 || !fs.existsSync(a0Path)) {
    return { status: "not-compared" };
  }
  const a0 = PNG.sync.read(fs.readFileSync(a0Path));
  const port = PNG.sync.read(screenshotBytes);
  const [left, right, top, bottom] = [17, 457, 30, 466];
  const blue = (image, x, y) => {
    const k = (y * 640 + x) * 4;
    return image.data[k + 2] > 150 && image.data[k] < 60 && image.data[k + 1] < 60;
  };
  const near = (image, x, y) => {
    for (let j = -2; j <= 2; j++) for (let i = -2; i <= 2; i++) if (blue(image, x + i, y + j)) return true;
    return false;
  };
  const counts = { a0_edge: 0, a0_edge_matched: 0, port_edge: 0, port_edge_matched: 0, port_lit: 0 };
  for (let y = top; y < bottom; y++) {
    for (let x = left; x < right; x++) {
      if (blue(a0, x, y)) {
        counts.a0_edge++;
        if (near(port, x, y)) counts.a0_edge_matched++;
      }
      if (blue(port, x, y)) {
        counts.port_edge++;
        if (near(a0, x, y)) counts.port_edge_matched++;
      } else {
        const k = (y * 640 + x) * 4;
        if (Math.max(port.data[k], port.data[k + 1], port.data[k + 2]) > 40) counts.port_lit++;
      }
    }
  }
  assert.ok(counts.a0_edge > 1000 && counts.port_edge > 1000, `holocube edges ${JSON.stringify(counts)}`);
  assert.ok(counts.a0_edge_matched / counts.a0_edge >= 0.99, `A0's edges off the port's ${JSON.stringify(counts)}`);
  assert.ok(counts.port_edge_matched / counts.port_edge >= 0.99, `the port's edges off A0's ${JSON.stringify(counts)}`);
  assert.ok(counts.port_lit < 2000, `a backdrop lights the empty view ${JSON.stringify(counts)}`);
  return { status: "compared", ...counts };
}

function verifyTacticalBackdrop(viewport, screenshotBytes, paletteResourceId) {
  if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
    return { status: "non-native-scale", bright_pixels_checked: 0 };
  }
  const screenshot = PNG.sync.read(screenshotBytes);
  const runtimeRoot = path.join(root, "data/base/ui/tactical-dll/TACTICAL3D/runtime");
  const manifest = JSON.parse(fs.readFileSync(path.join(runtimeRoot, "manifest.json"), "utf8"));
  const textureRecord = manifest.textures.find(({ identifier_kind: kind, id }) =>
    kind === "id" && id === 5030);
  const paletteRecord = manifest.textures.find(({ identifier_kind: kind, id }) =>
    kind === "id" && id === paletteResourceId);
  assert.ok(textureRecord && paletteRecord, "resource 5030 or its active palette is missing");
  const texture = fs.readFileSync(path.join(runtimeRoot, textureRecord.object));
  const palette = fs.readFileSync(path.join(runtimeRoot, paletteRecord.object));
  assert.equal(texture.subarray(0, 8).toString("ascii"), "ORTINDEX");
  assert.equal(palette.subarray(0, 8).toString("ascii"), "ORTPAL00");
  assert.equal(texture.readUInt32LE(8), 1);
  assert.equal(palette.readUInt32LE(8), 1);
  assert.equal(palette.readUInt32LE(12), paletteResourceId);
  const width = texture.readUInt32LE(12);
  const height = texture.readUInt32LE(16);
  const pixelCount = texture.readUInt32LE(28);
  assert.deepEqual([width, height, pixelCount], [440, 438, 440 * 438]);
  const indices = texture.subarray(36, 36 + pixelCount);
  const rgb = palette.subarray(16, 16 + 256 * 3);
  assert.equal(indices.length, pixelCount);
  assert.equal(rgb.length, 256 * 3);
  let brightPixels = 0;
  let matchingBrightPixels = 0;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const index = indices[y * width + x];
      const color = Array.from(rgb.subarray(index * 3, index * 3 + 3));
      if (Math.max(...color) < 16) continue;
      brightPixels++;
      const offset = ((29 + y) * screenshot.width + 18 + x) * 4;
      const actual = Array.from(screenshot.data.subarray(offset, offset + 3));
      if (actual.every((channel, index) => channel === color[index])) matchingBrightPixels++;
    }
  }
  assert.ok(brightPixels > 100, "resource 5030 lacks a meaningful bright-star sample");
  const matchRatio = matchingBrightPixels / brightPixels;
  assert.ok(matchRatio > 0.6,
    `resource 5030 bright-star match ratio was ${matchRatio.toFixed(3)}`);
  return {
    status: "source-bitmap-visible",
    resource_id: 5030,
    bright_pixels_checked: brightPixels,
    matching_bright_pixels: matchingBrightPixels,
    match_ratio: matchRatio,
  };
}

function verifyTacticalGreenBeam(viewport, beforeBytes, beamBytes) {
  const before = PNG.sync.read(beforeBytes);
  const beam = PNG.sync.read(beamBytes);
  assert.deepEqual([beam.width, beam.height], [before.width, before.height]);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const minX = Math.floor(offsetX + 16 * scale);
  const maxX = Math.ceil(offsetX + 460 * scale);
  const minY = Math.floor(offsetY + 28 * scale);
  const maxY = Math.ceil(offsetY + 467 * scale);
  let changedGreenPixels = 0;
  for (let y = minY; y < maxY; y++) {
    for (let x = minX; x < maxX; x++) {
      const offset = (y * beam.width + x) * 4;
      const red = beam.data[offset];
      const green = beam.data[offset + 1];
      const blue = beam.data[offset + 2];
      const changed = red !== before.data[offset]
        || green !== before.data[offset + 1]
        || blue !== before.data[offset + 2];
      if (changed && green > 96 && green > red * 1.5 && green > blue * 1.25) {
        changedGreenPixels++;
      }
    }
  }
  assert.ok(changedGreenPixels > 0, "Death Star beam produced no visible green aperture pixels");
  return { status: "visible", changed_green_pixels: changedGreenPixels };
}

function verifyStrategyBitmap(viewport, screenshotBytes, id, x0, y0, ignoredRects = [], keyBlack = false, covered = null) {
  if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
    return { status: "non-native-scale", pixels_checked: 0 };
  }
  const screenshot = PNG.sync.read(screenshotBytes);
  const source = decodeIndexedBmp(fs.readFileSync(path.join(
    root, `data/base/ui/strategy-dll/BMP/${id}.bmp`,
  )));
  let pixelsChecked = 0;
  for (let y = 0; y < source.height; y++) {
    for (let x = 0; x < source.width; x++) {
      if (ignoredRects.some((rect) => x >= rect.x && y >= rect.y
        && x < rect.x + rect.width && y < rect.y + rect.height)) continue;
      const color = source.pixel(x, y);
      if (color[0] < 32 && color[1] < 32 && color[2] > 192) continue;
      // Art keyed on its first stored pixel, black (the Display pages'
      // destination buttons 10764, 10765): the page shows there.
      if (keyBlack && color[0] === 0 && color[1] === 0 && color[2] === 0) continue;
      // A pixel another layer paints over (the keyed window frame's arcs).
      if (covered && covered(x, y)) continue;
      const actualOffset = ((y0 + y) * screenshot.width + x0 + x) * 4;
      assert.deepEqual(
        Array.from(screenshot.data.subarray(actualOffset, actualOffset + 3)), color,
        `strategy control ${id} pixel (${x}, ${y}) differs from its original BMP`,
      );
      pixelsChecked++;
    }
  }
  return { status: "source-bitmap-exact", resource_id: id, pixels_checked: pixelsChecked };
}

function verifyCommonBitmap(viewport, screenshotBytes, id, x0, y0, ignoredRects = []) {
  if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
    return { status: "non-native-scale", pixels_checked: 0 };
  }
  const screenshot = PNG.sync.read(screenshotBytes);
  const source = decodeIndexedBmp(fs.readFileSync(path.join(
    root, `data/base/ui/common-dll/BMP/${id}.bmp`,
  )));
  // The Game Options controls 10020..10057 are blitted keyed on their first
  // DIB pixel (bmp_cache::uses_first_pixel_key; A0 200-options).
  const key = id >= 10020 && id <= 10057 ? source.index(0, source.height - 1) : null;
  let pixelsChecked = 0;
  for (let y = 0; y < source.height; y++) {
    for (let x = 0; x < source.width; x++) {
      if (ignoredRects.some((rect) => x >= rect.x && y >= rect.y
        && x < rect.x + rect.width && y < rect.y + rect.height)) continue;
      if (key !== null && source.index(x, y) === key) continue;
      const color = source.pixel(x, y);
      const actualOffset = ((y0 + y) * screenshot.width + x0 + x) * 4;
      assert.deepEqual(
        Array.from(screenshot.data.subarray(actualOffset, actualOffset + 3)), color,
        `common control ${id} pixel (${x}, ${y}) differs from its original BMP`,
      );
      pixelsChecked++;
    }
  }
  return { status: "source-bitmap-exact", resource_id: id, pixels_checked: pixelsChecked };
}

function verifyTacticalTransparentOverlay(viewport, screenshotBytes, id, x0, y0) {
  if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
    return { status: "non-native-scale", pixels_checked: 0 };
  }
  const screenshot = PNG.sync.read(screenshotBytes);
  const source = decodeIndexedBmp(fs.readFileSync(path.join(
    root, `data/base/ui/tactical-dll/BMP/${id}.bmp`,
  )));
  let pixelsChecked = 0;
  for (let y = 0; y < source.height; y++) {
    for (let x = 0; x < source.width; x++) {
      const color = source.pixel(x, y);
      const blueMatte = color[0] < 32 && color[1] < 32 && color[2] > 192;
      const limeMatte = color[0] < 32 && color[1] > 192 && color[2] < 32;
      if (blueMatte || limeMatte) continue;
      const actualOffset = ((y0 + y) * screenshot.width + x0 + x) * 4;
      assert.deepEqual(
        Array.from(screenshot.data.subarray(actualOffset, actualOffset + 3)), color,
        `tactical overlay ${id} pixel (${x}, ${y}) differs from its original BMP`,
      );
      pixelsChecked++;
    }
  }
  assert.ok(pixelsChecked > 0, `tactical overlay ${id} has no inspectable pixels`);
  return { status: "source-bitmap-exact", resource_id: id, pixels_checked: pixelsChecked };
}

function verifyTacticalShell(viewport, screenshotBytes) {
  if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
    return { status: "non-native-scale", pixels_checked: 0 };
  }
  const screenshot = PNG.sync.read(screenshotBytes);
  const source = decodeIndexedBmp(fs.readFileSync(path.join(
    root, "data/base/ui/tactical-dll/BMP/1000.bmp",
  )));
  assert.equal(source.width, 640);
  assert.equal(source.height, 480);
  let pixelsChecked = 0;
  // Uncovered corner and bottom-edge pixels must be the original shell. The
  // aperture, group headers, selected panel, and controls are layered above it.
  const points = [];
  // The outermost two WebGL edge pixels are antialiased against the canvas
  // backing color; compare the first fully covered source pixels instead.
  for (let y = 2; y < 8; y++) {
    for (let x = 2; x < 8; x++) {
      points.push([x, y], [628 + x, y]);
    }
  }
  for (let x = 2; x < 638; x++) points.push([x, 476]);
  for (const [x, y] of points) {
    const offset = (y * 640 + x) * 4;
    assert.deepEqual(Array.from(screenshot.data.subarray(offset, offset + 3)), source.pixel(x, y),
      `tactical shell pixel (${x}, ${y}) differs from TACTICAL 1000`);
    pixelsChecked++;
  }
  return { status: "source-bitmap-exact", resource_id: 1000, pixels_checked: pixelsChecked };
}

function verifyTacticalApertureIsolation(viewport, beforeBytes, afterBytes, ignoreNativeRects = []) {
  const before = PNG.sync.read(beforeBytes);
  const after = PNG.sync.read(afterBytes);
  assert.equal(before.width, after.width);
  assert.equal(before.height, after.height);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const aperture = {
    x0: Math.round(offsetX + 16 * scale),
    y0: Math.round(offsetY + 28 * scale),
    x1: Math.round(offsetX + 460 * scale),
    y1: Math.round(offsetY + 467 * scale),
  };
  const ignored = ignoreNativeRects.map(({ x, y, width, height, reason }) => ({
    x0: Math.floor(offsetX + x * scale),
    y0: Math.floor(offsetY + y * scale),
    x1: Math.ceil(offsetX + (x + width) * scale),
    y1: Math.ceil(offsetY + (y + height) * scale),
    reason,
  }));
  let pixelsChecked = 0;
  for (let y = 0; y < before.height; y++) {
    for (let x = 0; x < before.width; x++) {
      if (x >= aperture.x0 && x < aperture.x1 && y >= aperture.y0 && y < aperture.y1) continue;
      if (ignored.some((rect) =>
        x >= rect.x0 && x < rect.x1 && y >= rect.y0 && y < rect.y1)) continue;
      const offset = (y * before.width + x) * 4;
      assert.deepEqual(
        Array.from(after.data.subarray(offset, offset + 4)),
        Array.from(before.data.subarray(offset, offset + 4)),
        `battlefield redraw escaped tactical aperture at (${x}, ${y})`,
      );
      pixelsChecked++;
    }
  }
  return {
    status: "isolated",
    pixels_checked: pixelsChecked,
    ...(ignoreNativeRects.length ? { ignored_native_rects: ignoreNativeRects } : {}),
  };
}

function tacticalApertureHash(viewport, screenshotBytes) {
  const screenshot = PNG.sync.read(screenshotBytes);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const x0 = Math.round(offsetX + 16 * scale);
  const y0 = Math.round(offsetY + 28 * scale);
  const x1 = Math.round(offsetX + 460 * scale);
  const y1 = Math.round(offsetY + 467 * scale);
  const hash = createHash("sha256");
  for (let y = y0; y < y1; y++) {
    hash.update(screenshot.data.subarray((y * screenshot.width + x0) * 4,
      (y * screenshot.width + x1) * 4));
  }
  return hash.digest("hex");
}

function tacticalProofModelHash(viewport, screenshotBytes) {
  const screenshot = PNG.sync.read(screenshotBytes);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const x0 = Math.floor(offsetX + 180 * scale);
  const y0 = Math.floor(offsetY + 80 * scale);
  const x1 = Math.ceil(offsetX + 440 * scale);
  const y1 = Math.ceil(offsetY + 320 * scale);
  const hash = createHash("sha256");
  for (let y = y0; y < y1; y++) {
    hash.update(screenshot.data.subarray((y * screenshot.width + x0) * 4,
      (y * screenshot.width + x1) * 4));
  }
  return hash.digest("hex");
}

function tacticalApertureChangedPixels(viewport, proofBytes, controlBytes) {
  const proof = PNG.sync.read(proofBytes);
  const control = PNG.sync.read(controlBytes);
  assert.equal(proof.width, control.width);
  assert.equal(proof.height, control.height);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const x0 = Math.floor(offsetX + 16 * scale);
  const y0 = Math.floor(offsetY + 28 * scale);
  const x1 = Math.ceil(offsetX + 460 * scale);
  const y1 = Math.ceil(offsetY + 476 * scale);
  let changed = 0;
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const offset = (y * proof.width + x) * 4;
      if (Math.abs(proof.data[offset] - control.data[offset]) > 2
        || Math.abs(proof.data[offset + 1] - control.data[offset + 1]) > 2
        || Math.abs(proof.data[offset + 2] - control.data[offset + 2]) > 2) {
        changed++;
      }
    }
  }
  return changed;
}

function tacticalModelRegionChangedPixels(
  viewport, proofBytes, controlBytes, center, logicalRadius = 12,
) {
  const proof = PNG.sync.read(proofBytes);
  const control = PNG.sync.read(controlBytes);
  assert.equal(proof.width, control.width);
  assert.equal(proof.height, control.height);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const radius = Math.ceil(logicalRadius * scale);
  const x0 = Math.max(0, Math.floor(center.x - radius));
  const y0 = Math.max(0, Math.floor(center.y - radius));
  const x1 = Math.min(proof.width, Math.ceil(center.x + radius));
  const y1 = Math.min(proof.height, Math.ceil(center.y + radius));
  let changed = 0;
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const offset = (y * proof.width + x) * 4;
      if (Math.abs(proof.data[offset] - control.data[offset]) > 2
        || Math.abs(proof.data[offset + 1] - control.data[offset + 1]) > 2
        || Math.abs(proof.data[offset + 2] - control.data[offset + 2]) > 2) {
        changed++;
      }
    }
  }
  return changed;
}

function verifyTacticalProofNegativeControls(results, factions, viewports) {
  for (const faction of factions) {
    for (const viewport of viewports) {
      const proof = results.find(({ id }) =>
        id === `tactical/${faction}/battle-entry/${viewport.id}`);
      const control = results.find(({ id }) =>
        id === `tactical/${faction}/battle-entry-proof-off/${viewport.id}`);
      assert.ok(proof && control, `missing tactical proof pair for ${faction}/${viewport.id}`);
      if (proof.status !== "pass" || control.status !== "pass") continue;
      const proofImage = fs.readFileSync(path.join(runDir, proof.id, "actual.png"));
      const controlImage = fs.readFileSync(path.join(runDir, control.id, "actual.png"));
      const changedPixels = tacticalApertureChangedPixels(viewport, proofImage, controlImage);
      const scale = Math.min(viewport.width / 640, viewport.height / 480);
      const minimumChanged = Math.ceil(1000 * scale * scale);
      const probe = {
        type: "tactical-3d-pixel-submission-negative-control",
        proof_fixture_code: proof.fixture_code,
        control_fixture_code: control.fixture_code,
        changed_aperture_pixels: changedPixels,
        minimum_changed_pixels: minimumChanged,
      };
      for (const result of [proof, control]) {
        result.probes.push(probe);
        if (changedPixels < minimumChanged) {
          result.status = "fail";
          result.error = `tactical proof/control aperture differs by only ${changedPixels} pixels`;
        }
        fs.writeFileSync(path.join(runDir, result.id, "result.json"),
          `${JSON.stringify(result, null, 2)}\n`);
      }
    }
  }
}

function verifyTacticalProductionParticipants(results, factions, viewports) {
  for (const faction of factions) {
    for (const viewport of viewports) {
      const production = results.find(({ id }) =>
        id === `tactical/${faction}/production-participants/${viewport.id}`);
      const control = results.find(({ id }) =>
        id === `tactical/${faction}/production-participants-3d-off/${viewport.id}`);
      assert.ok(production && control,
        `missing tactical production participant pair for ${faction}/${viewport.id}`);
      if (production.status !== "pass" || control.status !== "pass") continue;
      const productionImage = fs.readFileSync(path.join(runDir, production.id, "actual.png"));
      const controlImage = fs.readFileSync(path.join(runDir, control.id, "actual.png"));
      const changedPixels = tacticalApertureChangedPixels(
        viewport, productionImage, controlImage,
      );
      const scale = Math.min(viewport.width / 640, viewport.height / 480);
      const sceneLog = production.console.find(({ text }) =>
        text.includes("[tactical_3d] participant_scene"));
      const positionMatch = sceneLog?.text.match(/screen_positions=([^ ]+)/);
      assert.ok(positionMatch, "production participant scene omitted projected positions");
      const modelRegions = positionMatch[1].split(";").map((entry) => {
        const match = entry.match(/^(\d+):(-?[\d.]+),(-?[\d.]+)$/);
        assert.ok(match, `invalid projected participant position ${entry}`);
        const center = { x: Number(match[2]), y: Number(match[3]) };
        return {
          object_id: Number(match[1]),
          center,
          changed_pixels: tacticalModelRegionChangedPixels(
            viewport, productionImage, controlImage, center,
          ),
          minimum_changed_pixels: Math.ceil(2 * scale * scale),
        };
      });
      assert.deepEqual(modelRegions.map(({ object_id }) => object_id), [1, 2]);
      const fighterSceneLog = production.console.find(({ text }) =>
        text.includes("[tactical_3d] fighter_scene"));
      const fighterPositionMatch = fighterSceneLog?.text.match(/screen_positions=([^ ]+)/);
      assert.ok(fighterPositionMatch, "production fighter scene omitted projected positions");
      const fighterRegions = fighterPositionMatch[1].split(";").map((entry) => {
        const match = entry.match(/^(\d+):(-?[\d.]+),(-?[\d.]+)$/);
        assert.ok(match, `invalid projected fighter position ${entry}`);
        const center = { x: Number(match[2]), y: Number(match[3]) };
        return {
          object_id: Number(match[1]),
          center,
          changed_pixels: tacticalModelRegionChangedPixels(
            viewport, productionImage, controlImage, center, 4,
          ),
          minimum_changed_pixels: Math.ceil(scale * scale),
        };
      });
      assert.deepEqual(fighterRegions.map(({ object_id }) => object_id), [1001, 1002]);
      const minimumChanged = [...modelRegions, ...fighterRegions].reduce(
        (total, region) => total + region.minimum_changed_pixels, 0,
      );
      const probe = {
        type: "production-tactical-participant-rendering",
        resource_families: [2010, 2510],
        selected_lod_resources: [2012, 2512],
        changed_aperture_pixels: changedPixels,
        minimum_changed_pixels: minimumChanged,
        model_regions: modelRegions,
        fighter_regions: fighterRegions,
      };
      for (const result of [production, control]) result.probes.push(probe);
      if (changedPixels < minimumChanged
        || [...modelRegions, ...fighterRegions].some((region) =>
          region.changed_pixels < region.minimum_changed_pixels)) {
        production.status = "fail";
        control.status = "fail";
        production.error = "production participant framebuffer lacks visible source pixels";
        control.error = production.error;
      }
      for (const result of [production, control]) {
        fs.writeFileSync(path.join(runDir, result.id, "result.json"),
          `${JSON.stringify(result, null, 2)}\n`);
      }
    }
  }
}

function verifyTacticalLodFamily(results, factions, viewports) {
  const slugs = ["lod-close", "lod-medium", "lod-far"];
  for (const faction of factions) {
    for (const viewport of viewports) {
      const family = slugs.map((slug) => results.find(({ id }) =>
        id === `tactical/${faction}/${slug}/${viewport.id}`));
      assert.ok(family.every(Boolean), `missing tactical LOD family for ${faction}/${viewport.id}`);
      if (family.some(({ status }) => status !== "pass")) continue;
      const images = family.map(({ id }) =>
        fs.readFileSync(path.join(runDir, id, "actual.png")));
      const hashes = images.map((bytes) => tacticalApertureHash(viewport, bytes));
      assert.equal(new Set(hashes).size, 3,
        `tactical LOD views are not distinct for ${faction}/${viewport.id}`);
      const closeMediumChanged = tacticalApertureChangedPixels(viewport, images[0], images[1]);
      const mediumFarChanged = tacticalApertureChangedPixels(viewport, images[1], images[2]);
      assert.ok(closeMediumChanged > 100 && mediumFarChanged > 100,
        `tactical LOD selections changed too few aperture pixels for ${faction}/${viewport.id}`);
      const closeMediumIsolation = verifyTacticalApertureIsolation(viewport, images[0], images[1]);
      const mediumFarIsolation = verifyTacticalApertureIsolation(viewport, images[1], images[2]);
      const probe = {
        type: "tactical-3d-three-lod-selection-matrix",
        resources: [2560, 2561, 2562],
        aperture_hashes: hashes,
        close_medium_changed_pixels: closeMediumChanged,
        medium_far_changed_pixels: mediumFarChanged,
        close_medium_isolation: closeMediumIsolation,
        medium_far_isolation: mediumFarIsolation,
      };
      for (const result of family) {
        result.probes.push(probe);
        fs.writeFileSync(path.join(runDir, result.id, "result.json"),
          `${JSON.stringify(result, null, 2)}\n`);
      }
    }
  }
}

function verifyWireframeColor(viewport, onBytes, offBytes, faction) {
  const on = PNG.sync.read(onBytes);
  const off = PNG.sync.read(offBytes);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const x0 = Math.round(offsetX + 16 * scale);
  const y0 = Math.round(offsetY + 28 * scale);
  const x1 = Math.round(offsetX + 460 * scale);
  const y1 = Math.round(offsetY + 467 * scale);
  let changedPixels = 0;
  let factionColorPixels = 0;
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const offset = (y * on.width + x) * 4;
      const onPixel = Array.from(on.data.subarray(offset, offset + 3));
      const offPixel = Array.from(off.data.subarray(offset, offset + 3));
      if (onPixel.every((channel, index) => channel === offPixel[index])) continue;
      changedPixels++;
      const [red, green, blue] = onPixel;
      const factionColor = faction === "alliance"
        ? red > 80 && red > green * 2 && red > blue * 2
        : green > 80 && green > red * 2 && green > blue * 1.5;
      if (factionColor) factionColorPixels++;
    }
  }
  assert.ok(changedPixels > 0, `${faction} highlight changed no aperture pixels`);
  assert.ok(factionColorPixels > 0, `${faction} highlight used the wrong wireframe color`);
  return { status: "verified", changed_pixels: changedPixels,
    faction_color_pixels: factionColorPixels };
}

async function probeTactical(page, viewport, folder, stable) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const capture = async (name) => stableInteractionFrame(page, folder, name);
  const probes = [{ type: "tactical-shell", ...verifyTacticalShell(viewport, stable.bytes) }];
  probes.push({ type: "paused-control-bitmap",
    ...verifyTacticalBitmap(viewport, stable.bytes, 1061, 561, 308) });
  // A0 TAC-01-C003/C002: a battle opens with both side highlights off.
  probes.push({ type: "alliance-highlight-initially-off",
    ...verifyTacticalBitmap(viewport, stable.bytes, 1034, 515, 305) });
  probes.push({ type: "empire-highlight-initially-off",
    ...verifyTacticalBitmap(viewport, stable.bytes, 1036, 483, 305) });

  const outside = point(559, 319);
  await page.mouse.click(outside.x, outside.y);
  await page.waitForTimeout(80);
  const outsideFrame = await capture("pause-outside-edge");
  assert.equal(sha256(stable.bytes), sha256(outsideFrame), "outside-edge changed paused battle");
  probes.push({ type: "pause-outside-edge", unchanged: true });

  const pauseMatte = point(561.5, 308.5);
  await page.mouse.click(pauseMatte.x, pauseMatte.y);
  await page.waitForTimeout(80);
  const pauseMatteFrame = await capture("pause-transparent-matte");
  assert.equal(sha256(stable.bytes), sha256(pauseMatteFrame),
    "transparent pause-control matte pixel changed paused battle");
  probes.push({ type: "pause-transparent-matte", source_pixel: [1, 1], unchanged: true });

  const pause = point(575, 319);
  await page.mouse.click(pause.x, pause.y);
  await page.waitForTimeout(80);
  const resumed = await dynamicInteractionFrame(page, folder, "resumed");
  assert.notEqual(sha256(stable.bytes), sha256(resumed), "pause control did not resume battle");
  probes.push({ type: "resume", ...verifyTacticalBitmap(viewport, resumed, 1060, 561, 308) });
  // A0 TAC-01-C004 (the Alliance's Battle at Mon Calamari, running) and
  // TAC-03-C017 (the Empire's Battle at Yavin, running): the controls under
  // the panel (x 476..639, y 262..479), the running pause art 1060 among
  // them, match the running port pixel for pixel, but for A0 C004's pointer
  // (x 626..639, y 266..287). Neither shows "Battle Paused" (red ink at
  // x 17..139, y 30..49), nor does the running port.
  if (viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1) {
    const port = PNG.sync.read(resumed);
    const red = (png) => {
      let count = 0;
      for (let y = 30; y < 50; y++) for (let x = 17; x < 140; x++) {
        const i = (y * png.width + x) * 4;
        if (png.data[i] > 120 && png.data[i + 1] < 40 && png.data[i + 2] < 40) count++;
      }
      return count;
    };
    const running = { type: "running-controls-a0", port_paused_ink: red(port) };
    for (const [cell, pointer] of [["TAC-01-C004", [626, 266, 640, 288]], ["TAC-03-C017", null]]) {
      const file = path.join(root, `.artifacts/interface-parity/a0/${cell}/original.png`);
      if (!fs.existsSync(file)) continue;
      const a0 = PNG.sync.read(fs.readFileSync(file));
      let differing = 0;
      for (let y = 262; y < 480; y++) for (let x = 476; x < 640; x++) {
        if (pointer && x >= pointer[0] && x < pointer[2] && y >= pointer[1] && y < pointer[3]) continue;
        const i = (y * 640 + x) * 4;
        if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[i + c])) differing++;
      }
      running[cell] = { differing, a0_paused_ink: red(a0) };
    }
    probes.push(running);
    assert.equal(running.port_paused_ink, 0, JSON.stringify(running));
    for (const cell of ["TAC-01-C004", "TAC-03-C017"]) {
      if (!running[cell]) continue;
      assert.deepEqual(running[cell], { differing: 0, a0_paused_ink: 0 }, JSON.stringify(running));
    }
  }
  await page.mouse.click(pause.x, pause.y);
  await page.waitForTimeout(80);
  // Then the pointer leaves the control: resting on it, the shared tooltip
  // (win_tooltip.rs, "Resume Battle") would show between the two frames.
  // The release must reach the game first, or the move cancels the click.
  await page.mouse.move(2, 2);
  await page.waitForTimeout(80);
  const pausedAgain = await capture("paused-again");
  probes.push({ type: "pause", ...verifyTacticalBitmap(viewport, pausedAgain, 1061, 561, 308) });
  await page.waitForTimeout(250);
  const pauseStable = await capture("pause-stable");
  assert.equal(sha256(pausedAgain), sha256(pauseStable), "battle continued after re-pause");
  probes.push({ type: "paused-stability", stable: true,
    screenshot_sha256: sha256(pauseStable) });

  const allianceHighlight = point(530, 318);
  await page.mouse.click(allianceHighlight.x, allianceHighlight.y);
  await page.waitForTimeout(80);
  const highlightOn = await capture("alliance-highlight-on");
  assert.notEqual(sha256(pausedAgain), sha256(highlightOn), "Alliance highlight did not toggle");
  assert.notEqual(tacticalApertureHash(viewport, pauseStable),
    tacticalApertureHash(viewport, highlightOn), "Alliance battle wireframe did not light");
  probes.push({ type: "alliance-highlight-on",
    ...verifyTacticalBitmap(viewport, highlightOn, 1035, 515, 305),
    wireframe: verifyWireframeColor(viewport, highlightOn, pauseStable, "alliance") });
  await page.mouse.click(allianceHighlight.x, allianceHighlight.y);
  await page.waitForTimeout(80);
  const allianceRestored = await capture("alliance-highlight-restored");
  assert.equal(sha256(pauseStable), sha256(allianceRestored),
    "Alliance highlight did not restore the paused display");
  probes.push({ type: "alliance-highlight-restored",
    ...verifyTacticalBitmap(viewport, allianceRestored, 1034, 515, 305) });

  const empireHighlight = point(498, 318);
  await page.mouse.click(empireHighlight.x, empireHighlight.y);
  await page.waitForTimeout(80);
  const empireOn = await capture("empire-highlight-on");
  assert.notEqual(sha256(allianceRestored), sha256(empireOn), "Empire highlight did not toggle");
  assert.notEqual(tacticalApertureHash(viewport, allianceRestored),
    tacticalApertureHash(viewport, empireOn), "Empire battle wireframe did not light");
  probes.push({ type: "empire-highlight-on",
    ...verifyTacticalBitmap(viewport, empireOn, 1037, 483, 305),
    wireframe: verifyWireframeColor(viewport, empireOn, allianceRestored, "empire") });
  await page.mouse.click(empireHighlight.x, empireHighlight.y);
  await page.waitForTimeout(80);
  const highlightsRestored = await capture("highlights-restored");
  assert.equal(sha256(pauseStable), sha256(highlightsRestored),
    "Empire highlight did not restore the paused display");
  probes.push({ type: "empire-highlight-restored",
    ...verifyTacticalBitmap(viewport, highlightsRestored, 1036, 483, 305) });

  const zoomInMatte = point(487.5, 344.5);
  await page.mouse.click(zoomInMatte.x, zoomInMatte.y);
  await page.waitForTimeout(80);
  const zoomInMatteFrame = await capture("zoom-in-transparent-matte");
  assert.equal(sha256(highlightsRestored), sha256(zoomInMatteFrame),
    "transparent zoom-in matte pixel changed paused battle");
  probes.push({ type: "zoom-in-transparent-matte", source_pixel: [1, 1], unchanged: true });

  const zoomIn = point(503, 355);
  await page.mouse.move(zoomIn.x, zoomIn.y);
  await page.mouse.down({ button: "left" });
  await page.waitForTimeout(80);
  const zoomInPressed = await capture("zoom-in-pressed");
  probes.push({ type: "zoom-in-pressed",
    ...verifyTacticalBitmap(viewport, zoomInPressed, 1045, 491, 343) });
  await page.mouse.up({ button: "left" });
  await page.waitForTimeout(80);
  const zoomed = await capture("zoomed-in");
  assert.notEqual(sha256(highlightsRestored), sha256(zoomed), "zoom-in control did not redraw battle");
  probes.push({ type: "zoom-in", ...verifyTacticalBitmap(viewport, zoomed, 1044, 491, 343),
    aperture_isolation: verifyTacticalApertureIsolation(viewport, highlightsRestored, zoomed) });

  const zoomOutMatte = point(604.5, 344.5);
  await page.mouse.click(zoomOutMatte.x, zoomOutMatte.y);
  await page.waitForTimeout(80);
  const zoomOutMatteFrame = await capture("zoom-out-transparent-matte");
  assert.equal(sha256(zoomed), sha256(zoomOutMatteFrame),
    "transparent zoom-out matte pixel changed zoomed battle");
  probes.push({ type: "zoom-out-transparent-matte", source_pixel: [1, 1], unchanged: true });

  const zoomOut = point(619, 355);
  await page.mouse.move(zoomOut.x, zoomOut.y);
  await page.mouse.down({ button: "left" });
  await page.waitForTimeout(80);
  const zoomOutPressed = await capture("zoom-out-pressed");
  probes.push({ type: "zoom-out-pressed",
    ...verifyTacticalBitmap(viewport, zoomOutPressed, 1047, 607, 343) });
  await page.mouse.up({ button: "left" });
  await page.waitForTimeout(80);
  const zoomRestored = await capture("zoom-restored");
  assert.equal(sha256(highlightsRestored), sha256(zoomRestored),
    "zoom-out did not restore the original paused battlefield");
  probes.push({ type: "zoom-out", ...verifyTacticalBitmap(viewport, zoomRestored, 1046, 607, 343),
    aperture_isolation: verifyTacticalApertureIsolation(viewport, zoomed, zoomRestored) });
  return probes;
}

async function probeTacticalLodJourney(page, viewport, folder, stable) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const capture = async (name) => stableInteractionFrame(page, folder, name);
  const clickRepeatedly = async (location, count) => {
    for (let index = 0; index < count; index++) {
      await page.mouse.click(location.x, location.y);
      await page.waitForTimeout(80);
    }
  };
  const zoomOut = point(619, 355);
  const zoomIn = point(503, 355);

  await clickRepeatedly(zoomOut, 1);
  const medium = await capture("lod-journey-medium");
  await clickRepeatedly(zoomOut, 5);
  const far = await capture("lod-journey-far");
  await clickRepeatedly(zoomIn, 1);
  const returnMedium = await capture("lod-journey-return-medium");
  // One additional authentic click reaches the existing 2.0 clamp exactly,
  // avoiding reciprocal-f32 drift that is visible at letterboxed resolution.
  await clickRepeatedly(zoomIn, 6);
  const restoredClose = await capture("lod-journey-restored-close");

  const apertureHashes = [stable.bytes, medium, far, returnMedium, restoredClose]
    .map((bytes) => tacticalApertureHash(viewport, bytes));
  const modelHashes = [stable.bytes, medium, far, returnMedium, restoredClose]
    .map((bytes) => tacticalProofModelHash(viewport, bytes));
  assert.equal(new Set(modelHashes.slice(0, 3)).size, 3,
    "same-renderer close, medium, and far journey views are not distinct");
  assert.equal(modelHashes[4], modelHashes[0],
    "same-renderer LOD journey did not restore its exact initial model view");
  const transitions = [
    [stable.bytes, medium],
    [medium, far],
    [far, returnMedium],
    [returnMedium, restoredClose],
  ];
  const provisionalTextMasks = [
    { x: 65, y: 4, width: 18, height: 18, reason: "provisional task-force number" },
    { x: 490, y: 35, width: 145, height: 20, reason: "provisional selected-ship label" },
    { x: 490, y: 208, width: 145, height: 22, reason: "provisional orders label" },
  ];
  const exactNativeScale = Number.isInteger(scale);
  const isolationMasks = exactNativeScale ? [] : provisionalTextMasks;
  const isolation = transitions.map(([before, after]) =>
    verifyTacticalApertureIsolation(viewport, before, after, isolationMasks));
  assert.ok(isolation.every(({ status }) => status === "isolated"),
    "same-renderer LOD journey changed pixels outside the tactical aperture");
  const restoredFullFrameExact = sha256(restoredClose) === sha256(stable.bytes);
  if (exactNativeScale) {
    assert.equal(restoredFullFrameExact, true,
      "native same-renderer LOD journey did not restore its exact initial frame");
  }

  return [{
    type: "tactical-3d-same-renderer-lod-journey",
    controls: ["zoom-out", "zoom-in"],
    expected_resources: [2560, 2561, 2562, 2561, 2560],
    aperture_hashes: apertureHashes,
    model_crop_native_rect: { x: 180, y: 80, width: 260, height: 240 },
    model_crop_hashes: modelHashes,
    aperture_isolation: isolation,
    restored_model_exact: true,
    restored_full_frame_exact: restoredFullFrameExact,
    native_full_frame_asserted: exactNativeScale,
  }];
}

async function probeTacticalCameraJourney(page, viewport, folder, stable) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const capture = async (name) => stableInteractionFrame(page, folder, name);
  const press = async (name, location, resource, resourceX, resourceY, occluders = []) => {
    await page.mouse.move(location.x, location.y);
    await page.mouse.down({ button: "left" });
    await page.waitForTimeout(80);
    const pressed = await capture(`${name}-pressed`);
    const bitmap = verifyTacticalBitmap(
      viewport, pressed, resource, resourceX, resourceY, occluders,
    );
    await page.mouse.up({ button: "left" });
    await page.waitForTimeout(80);
    const released = await capture(name);
    return { name, bitmap, released };
  };

  const initialModel = tacticalProofModelHash(viewport, stable.bytes);
  const transitions = [];
  transitions.push(await press("camera-zoom-in", point(503, 355), 1045, 491, 343));
  transitions.push(await press("camera-zoom-out", point(619, 355), 1047, 607, 343));
  const up = { id: 1052, x: 539, y: 338 };
  const down = { id: 1055, x: 540, y: 424 };
  const target = { id: 1058, x: 549, y: 391 };
  transitions.push(await press(
    "camera-left", point(506, 402), 1049, 497, 381, [up, down, target],
  ));
  transitions.push(await press(
    "camera-right", point(615, 402), 1051, 582, 381, [up, down, target],
  ));
  transitions.push(await press(
    "camera-up", point(560, 348), 1053, 539, 338, [target],
  ));
  transitions.push(await press("camera-down", point(561, 456), 1056, 540, 424));
  transitions.push(await press("camera-target", point(560, 402), 1059, 549, 391));
  // A0 TAC-04-C011 (Battle at Mon Calamari, the Victory Destroyer selected,
  // then the centre of the camera sphere, 1058/1059): the subject's green
  // box is centred at (237, 247), the battle view's middle. The port's
  // selected hull, after the same control, sits on the same point.
  const centreA0 = path.join(root, ".artifacts/interface-parity/a0/TAC-04-C011/original.png");
  let centre = null;
  if (viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1
    && fs.existsSync(centreA0)) {
    const green = (image, x, y) => {
      const k = (y * 640 + x) * 4;
      return image.data[k + 1] > 150 && image.data[k] < 80 && image.data[k + 2] < 80;
    };
    // The port's selected marker takes its side's colour: green for the
    // Alliance's hull, red for the Empire's.
    const red = (image, x, y) => {
      const k = (y * 640 + x) * 4;
      return image.data[k] > 150 && image.data[k + 1] < 80 && image.data[k + 2] < 80;
    };
    const middle = (image, [x0, x1, y0, y1], lit = green) => {
      let [left, right, top, bottom] = [Infinity, -Infinity, Infinity, -Infinity];
      for (let y = y0; y < y1; y++) {
        for (let x = x0; x < x1; x++) {
          if (!lit(image, x, y)) continue;
          [left, right, top, bottom] = [Math.min(left, x), Math.max(right, x), Math.min(top, y), Math.max(bottom, y)];
        }
      }
      return { x: (left + right) / 2, y: (top + bottom) / 2 };
    };
    const a0 = middle(PNG.sync.read(fs.readFileSync(centreA0)), [17, 457, 30, 466]);
    // The port's selected marker without its shield and hull bars.
    const port = middle(PNG.sync.read(transitions.at(-1).released), [222, 252, 237, 256],
      (image, x, y) => green(image, x, y) || red(image, x, y));
    centre = { a0, port };
    assert.ok(Math.abs(a0.x - port.x) <= 1 && Math.abs(a0.y - port.y) <= 1,
      `the centred subject ${JSON.stringify(centre)}`);
  }

  const modelHashes = [initialModel, ...transitions.map(({ released }) =>
    tacticalProofModelHash(viewport, released))];
  assert.ok(new Set(modelHashes).size >= 4,
    "source camera journey did not produce distinct rendered model views");
  return transitions.map(({ name, bitmap }, index) => ({
    type: "source-camera-control",
    control: name,
    pressed_bitmap: bitmap,
    model_crop_sha256: modelHashes[index + 1],
    ...(name === "camera-target" && centre ? { centre_a0: centre } : {}),
  }));
}

function parseTacticalScenePositions(line, label) {
  const match = line?.text.match(/screen_positions=([^ ]+)/);
  assert.ok(match, `${label} omitted projected positions`);
  return new Map(match[1].split(";").map((entry) => {
    const position = entry.match(/^(\d+):(-?[\d.]+),(-?[\d.]+)$/);
    assert.ok(position, `invalid ${label} projected position ${entry}`);
    return [Number(position[1]), { x: Number(position[2]), y: Number(position[3]) }];
  }));
}

// A0 TAC-01-C003's battle (one Medium Transport against one Victory
// Destroyer, extent 103, FUN_005ab650): the bounding box, in 640 x 480
// canvas pixels, of each hull's lit pixels around its projected point.
function probeTacticalHullScale(viewport, folder, stable, consoleLines, ready) {
  const participantLine = consoleLines.find(({ text }) =>
    text.includes("[tactical_3d] participant_scene"));
  const positions = parseTacticalScenePositions(participantLine, "hull-scale scene");
  const image = PNG.sync.read(stable.bytes);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const boxes = [];
  for (const [objectId, point] of positions) {
    let minX = Infinity; let minY = Infinity; let maxX = -Infinity; let maxY = -Infinity;
    const radius = Math.round(40 * scale);
    for (let y = Math.max(0, Math.round(point.y) - radius);
      y < Math.min(image.height, Math.round(point.y) + radius); y += 1) {
      for (let x = Math.max(0, Math.round(point.x) - radius);
        x < Math.min(image.width, Math.round(point.x) + radius); x += 1) {
        const offset = (y * image.width + x) * 4;
        const [red, green, blue] = [image.data[offset], image.data[offset + 1], image.data[offset + 2]];
        // Hull pixels are lit grey; stars are single points and the cube
        // edges pure blue.
        const grey = red > 60 && green > 60 && blue > 60
          && Math.abs(red - green) < 60 && Math.abs(green - blue) < 80;
        if (!grey) continue;
        minX = Math.min(minX, x); maxX = Math.max(maxX, x);
        minY = Math.min(minY, y); maxY = Math.max(maxY, y);
      }
    }
    assert.ok(Number.isFinite(minX), `hull ${objectId} drew no lit pixels`);
    boxes.push({
      object_id: objectId,
      canvas_x: (point.x - offsetX) / scale,
      canvas_y: (point.y - offsetY) / scale,
      width_px: (maxX - minX + 1) / scale,
      height_px: (maxY - minY + 1) / scale,
    });
  }
  console.log(`[hull_scale] ${JSON.stringify(boxes)}`);
  fs.writeFileSync(path.join(folder, "hull-scale.json"), JSON.stringify(boxes, null, 2));
  return [{
    type: "hull-scale-a0-c003",
    extent: ready.battle_extent ?? null,
    boxes,
    screenshot_sha256: sha256(stable.bytes),
  }];
}

async function probeTacticalProductionParticipants(
  page, viewport, folder, stable, consoleLines, faction, ready,
) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const probes = probeTacticalSelectedDamagePresentation(
    viewport,
    folder,
    stable,
    ready,
    { hull: ready.selected_ship?.hull_max, shield: ready.selected_ship?.shield_max },
    { filename: "selected-healthy.png", type: "source-mapped-selected-capital-healthy" },
  );

  const assignedTaskForces = new Set(ready.participants
    .filter(({ kind, faction: owner, active, task_force }) =>
      kind === "capital-ship" && owner === faction && active && Number.isInteger(task_force))
    .map(({ task_force }) => task_force));
  const assignedFighterGroups = new Set(ready.participants
    .filter(({ kind, faction: owner, active, fighter_group }) =>
      kind === "fighter-group" && owner === faction && active && Number.isInteger(fighter_group))
    .map(({ fighter_group }) => fighter_group));
  const unassignedTaskForces = [];
  for (let index = 0; index < 8; index++) {
    if (assignedTaskForces.has(index)) continue;
    unassignedTaskForces.push({
      slot: index + 1,
      ...verifyTacticalBitmap(viewport, stable.bytes, 1007, 60 + index * 29, 2), // A0 TAC-02-C002
    });
  }
  const unassignedFighterGroups = [];
  for (let index = 0; index < 4; index++) {
    if (assignedFighterGroups.has(index)) continue;
    unassignedFighterGroups.push({
      slot: index + 1,
      ...verifySquadronButton(viewport, stable.bytes, 1020, 347 + index * 29, 2),
    });
  }
  assert.ok(unassignedTaskForces.length > 0 && unassignedFighterGroups.length > 0,
    "production participant fixture omitted unassigned group states");
  const unassignedTaskIndex = unassignedTaskForces[0].slot - 1;
  const unassignedTask = point(72 + unassignedTaskIndex * 29, 10);
  await page.mouse.move(unassignedTask.x, unassignedTask.y);
  await page.waitForTimeout(80);
  const hovered = await stableInteractionFrame(page, folder, "task-force-unassigned-hover");
  assert.equal(sha256(hovered), sha256(stable.bytes),
    "unassigned task-force hover altered the original disabled bitmap state");
  const transitionLogCount = consoleLines.filter(({ text }) =>
    text.includes("[tactical_groups]")).length;
  await page.mouse.down({ button: "left" });
  await page.waitForTimeout(80);
  const held = await stableInteractionFrame(page, folder, "task-force-unassigned-held");
  await page.mouse.up({ button: "left" });
  await page.waitForTimeout(80);
  const released = await stableInteractionFrame(page, folder, "task-force-unassigned-released");
  assert.equal(sha256(held), sha256(stable.bytes),
    "unassigned task-force press escaped the disabled bitmap state");
  assert.equal(sha256(released), sha256(stable.bytes),
    "unassigned task-force release changed tactical selection");
  assert.equal(consoleLines.filter(({ text }) => text.includes("[tactical_groups]")).length,
    transitionLogCount, "unassigned task-force input emitted a selection transition");
  probes.push({
    type: "task-force-unassigned-hover-disabled",
    task_force_resource: 1007,
    fighter_group_resource: 1020,
    assigned_task_forces: [...assignedTaskForces].sort(),
    assigned_fighter_groups: [...assignedFighterGroups].sort(),
    task_forces: unassignedTaskForces,
    fighter_groups: unassignedFighterGroups,
    hover_sha256: sha256(hovered),
    held_sha256: sha256(held),
    released_sha256: sha256(released),
    state_unchanged: true,
    transition_log_count: transitionLogCount,
  });

  const participantLine = consoleLines.find(({ text }) =>
    text.includes("[tactical_3d] participant_scene"));
  const positions = parseTacticalScenePositions(participantLine, "capital participant scene");
  const playerObjectId = faction === "alliance" ? 1 : 2;
  const targetObjectId = faction === "alliance" ? 2 : 1;
  const player = positions.get(playerObjectId);
  const target = positions.get(targetObjectId);
  assert.ok(player && target, "production participant interaction positions are incomplete");

  // Empty space beside the player's ship. FUN_005d9640's camera puts the
  // Alliance ship near the view's left edge (A0 TAC-01-C003: (28, 428)).
  await page.mouse.click(player.x + 80, player.y, { button: "left" });
  await page.waitForTimeout(80);
  const deselected = await stableInteractionFrame(page, folder, "production-deselection");
  await page.mouse.click(player.x, player.y, { button: "left" });
  await page.waitForTimeout(80);
  const selected = await stableInteractionFrame(page, folder, "production-selection");
  await page.mouse.click(target.x, target.y, { button: "right" });
  await page.waitForTimeout(80);
  const targeted = await stableInteractionFrame(page, folder, "production-target");
  assert.notEqual(sha256(deselected), sha256(stable.bytes),
    "projected capital deselection produced no stable framebuffer change");
  assert.equal(sha256(selected), sha256(stable.bytes),
    "projected capital reselection did not restore the deterministic selection frame");
  assert.notEqual(sha256(targeted), sha256(selected),
    "projected capital targeting produced no stable framebuffer change");

  const selectionLog = consoleLines.find(({ text }) =>
    text.includes(`[tactical_3d] selection object_id=${playerObjectId}`));
  const focusLog = consoleLines.find(({ text }) =>
    text.includes(`[tactical_3d] focus source_object_ids=${playerObjectId} `)
      && text.includes(`target_object_id=${targetObjectId}`));
  assert.match(selectionLog?.text || "", /source_projection=true/,
    "selection did not use the source projection");
  assert.match(focusLog?.text || "", /source_projection=true/,
    "focus targeting did not use the source projection");
  probes.push({
    type: "source-projected-tactical-interaction",
    player_object_id: playerObjectId,
    target_object_id: targetObjectId,
    deselected_sha256: sha256(deselected),
    selected_sha256: sha256(selected),
    targeted_sha256: sha256(targeted),
  });

  // The side highlights (FUN_0059f380): A0 TAC-04-C006 boxes each Alliance
  // ship in pure red, TAC-04-C007 and a0-vm/research-display/32 each
  // Imperial ship in pure green, one pixel wide. Toggle both, count the
  // pure pixels in the battle view, toggle both back.
  const highlightPixels = (bytes) => {
    const png = PNG.sync.read(bytes);
    const counts = { red: 0, green: 0 };
    for (let y = Math.round(offsetY + 30 * scale); y < Math.round(offsetY + 468 * scale); y++) {
      for (let x = Math.round(offsetX + 16 * scale); x < Math.round(offsetX + 466 * scale); x++) {
        const i = (y * png.width + x) * 4;
        const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
        if (r === 255 && g === 0 && b === 0) counts.red++;
        if (r === 0 && g === 255 && b === 0) counts.green++;
      }
    }
    return counts;
  };
  const before = highlightPixels(targeted);
  const alliance = point(530, 318);
  const empire = point(498, 318);
  await page.mouse.click(alliance.x, alliance.y);
  await page.waitForTimeout(300);
  await page.mouse.click(empire.x, empire.y);
  await page.waitForTimeout(300);
  await page.mouse.move(point(470, 200).x, point(470, 200).y);
  const lit = await stableInteractionFrame(page, folder, "production-highlights-on");
  const after = highlightPixels(lit);
  if (scale === 1) {
    assert.ok(after.red > before.red + 20 && after.green > before.green + 20,
      `the highlights draw red and green boxes: ${JSON.stringify({ before, after })}`);
  }
  await page.mouse.click(alliance.x, alliance.y);
  await page.waitForTimeout(300);
  await page.mouse.click(empire.x, empire.y);
  await page.waitForTimeout(300);
  probes.push({ type: "side-highlight-boxes", before, after });
  return probes;
}

function parseFighterSceneResources(line) {
  const match = line?.text.match(/resources=([^ ]+)/);
  assert.ok(match, "fighter scene omitted selected resources");
  return match[1].split(",").map((entry) => {
    const fields = entry.match(/^(\d+):(\d+):(Close|Far|Indicator):([\d.]+)$/);
    assert.ok(fields, `invalid fighter scene resource ${entry}`);
    return {
      object_id: Number(fields[1]),
      resource_id: Number(fields[2]),
      detail: fields[3].toLowerCase(),
      view_span: Number(fields[4]),
    };
  });
}

async function probeTacticalFighterDetailJourney(
  page, viewport, folder, stable, consoleLines, ready,
) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const zoomIn = point(503, 355);
  const zoomOut = point(619, 355);
  const capture = async (name) => stableInteractionFrame(page, folder, name);
  const clickRepeatedly = async (location, count) => {
    for (let index = 0; index < count; index++) {
      await page.mouse.click(location.x, location.y);
      await page.waitForTimeout(40);
    }
  };

  await clickRepeatedly(zoomIn, 14);
  const far = await capture("fighter-detail-far");
  await clickRepeatedly(zoomIn, 7);
  const close = await capture("fighter-detail-close");
  await clickRepeatedly(zoomOut, 7);
  const returnFar = await capture("fighter-detail-return-far");
  await clickRepeatedly(zoomOut, 11);
  const returnIndicator = await capture("fighter-detail-return-indicator");

  const sceneLines = consoleLines.filter(({ text }) =>
    text.includes("[tactical_3d] fighter_scene"));
  assert.equal(sceneLines.length, 9,
    "fighter detail journey emitted the wrong number of state transitions");
  const states = sceneLines.map(parseFighterSceneResources);
  const details = states.map((state) => state.map(({ detail }) => detail));
  // Which group crosses each threshold first depends on its distance from
  // the camera. Under FUN_005d9640's camera the Alliance view has group 1001
  // nearer, the Imperial view group 1002.
  const nearerFirst = [
    ["indicator", "indicator"],
    ["far", "indicator"],
    ["far", "far"],
    ["close", "far"],
    ["close", "close"],
    ["close", "far"],
    ["far", "far"],
    ["far", "indicator"],
    ["indicator", "indicator"],
  ];
  assert.deepEqual(details, ready.faction === "empire"
    ? nearerFirst.map(([first, second]) => [second, first])
    : nearerFirst);
  const fighterParticipants = ready.participants.filter(({ kind }) => kind === "fighter-group");
  const activeResources = new Map(fighterParticipants.map((participant, index) => [1001 + index, {
    close: participant.active_close_resource,
    far: participant.active_far_resource,
    indicator: participant.active_indicator_resource,
  }]));
  for (const state of states) {
    for (const entry of state) {
      assert.equal(entry.resource_id, activeResources.get(entry.object_id)?.[entry.detail],
        `fighter ${entry.object_id} used the wrong ${entry.detail} group resource`);
    }
  }
  for (const state of states) {
    for (const { detail, view_span: viewSpan } of state) {
      if (detail === "close") assert.ok(viewSpan < 5);
      if (detail === "far") assert.ok(viewSpan >= 5 && viewSpan < 10);
      if (detail === "indicator") assert.ok(viewSpan >= 10);
    }
  }

  const frames = [stable.bytes, far, close, returnFar, returnIndicator];
  const apertureHashes = frames.map((bytes) => tacticalApertureHash(viewport, bytes));
  assert.equal(new Set(apertureHashes.slice(0, 3)).size, 3,
    "fighter indicator, far, and close views did not produce distinct framebuffers");
  const transitions = frames.slice(1).map((after, index) =>
    verifyTacticalApertureIsolation(viewport, frames[index], after));
  assert.ok(transitions.every(({ status }) => status === "isolated"),
    "fighter detail journey changed pixels outside the tactical aperture");
  return [{
    type: "production-tactical-fighter-detail-journey",
    controls: ["zoom-in", "zoom-out"],
    executable_functions: ["FUN_005c7150", "FUN_005c63f0", "0x005d4af0"],
    thresholds: { close_below: 5, far_below: 10, indicator_at_or_above: 10 },
    states,
    aperture_hashes: apertureHashes,
    aperture_isolation: transitions,
    family_loads: 2,
  }];
}

async function probeTacticalEffectPresentation(folder, stable, consoleLines, ready) {
  const firstTarget = ready.faction === "alliance" ? 0 : 1;
  const expected = [
    { target: firstTarget, resource_base: 3060, resource_id: 3062, frame: 2,
      frame_count: 6, source_size: [32, 32] },
    { target: firstTarget + 1, resource_base: 3180, resource_id: 3182, frame: 2,
      frame_count: 6, source_size: [32, 32] },
    { target: firstTarget + 2, resource_base: 3120, resource_id: 3123, frame: 3,
      frame_count: 7, source_size: [32, 32] },
    { target: firstTarget + 3, resource_base: 3240, resource_id: 3247, frame: 7,
      frame_count: 16, source_size: [64, 32] },
    { target: firstTarget + 4, resource_base: 3300, resource_id: 3302, frame: 2,
      frame_count: 7, source_size: [32, 32] },
    { target: firstTarget + 5, resource_base: 3360, resource_id: 3368, frame: 8,
      frame_count: 16, source_size: [64, 64] },
  ];
  assert.deepEqual(ready.effects, expected,
    "effect fixture did not expose the six exact source-selected frames");

  const loadLines = consoleLines.filter(({ text }) =>
    text.includes("[tactical_3d] effect_loaded"));
  assert.equal(loadLines.length, expected.length,
    "effect fixture did not load exactly six source frames");
  const loadedResources = loadLines.map(({ text }) => {
    const match = text.match(/resource=(\d+)/);
    assert.ok(match, "effect load omitted its resource identity");
    assert.match(text,
      /palette_selector=\d+ palette_resource_id=\d+ texture_filter=nearest transparent_index=\d+ source=FUN_005d38d0/,
      "effect load omitted its palette, transparency, filter, or source contract");
    return Number(match[1]);
  }).sort((left, right) => left - right);
  assert.deepEqual(loadedResources,
    expected.map(({ resource_id: resourceId }) => resourceId).sort((left, right) => left - right));

  const sceneLines = consoleLines.filter(({ text }) =>
    text.includes("[tactical_3d] effect_scene"));
  assert.equal(sceneLines.length, 1,
    "paused effect fixture emitted the wrong number of scene states");
  assert.match(sceneLines[0].text,
    /requested=6 rendered=6 .* frame_seconds=0\.1 source_positions=true source=FUN_005d39a0,FUN_005d3e90,FUN_005d41a0/,
    "effect scene omitted its target, timing, or executable contract");
  for (const { target, resource_id: resourceId } of expected) {
    assert.match(sceneLines[0].text, new RegExp(`${target + 1}:${resourceId}:\\d+:`),
      `effect ${resourceId} was not attached to source object ${target + 1}`);
  }
  fs.writeFileSync(path.join(folder, "effect-families.png"), stable.bytes);
  return [{
    type: "source-traced-tactical-impact-effects",
    executable_functions: [
      "FUN_005a7500", "FUN_005d39a0", "FUN_005d3e90", "FUN_005d41a0",
    ],
    frame_seconds: 0.1,
    transparency: "type-303-border-index",
    effects: expected,
  }];
}

async function probeTacticalDetailEscortPresentation(
  page, viewport, folder, stable, consoleLines, ready,
) {
  const selected = ready.selected_ship;
  assert.ok(selected, "detail fixture omitted its selected command hull");
  assert.ok(selected.contents.length > 0,
    "selected command hull omitted its assigned personnel contents");
  // FUN_005e4a30's list: each unit's FUN_00597610 icon from (495, 154)
  // every 32 pixels, the character first, then its regiments
  // (FUN_0040b0e0); four overflow the 124-wide list into its bar (A0 VM
  // a0-vm/coruscant-676/05: arrows 1351 at 495 and 1353 at 599, the thumb
  // 1355 at 515 and 1357 at 591).
  const content = selected.contents[0];
  assert.equal(selected.contents.length, 4, JSON.stringify(selected.contents));
  assert.ok(selected.contents.every(({ icon_id: icon }) => icon), JSON.stringify(selected.contents));
  // The icons are colour-matched to the battle palette: their entries
  // 160..245 take its nearest colours (checked against the VM below).
  const matched = (index) => index >= 160 && index <= 245;
  const contentProof = selected.contents.slice(0, 3).map((each, index) =>
    verifyTacticalBitmap(viewport, stable.bytes, each.icon_id, 495 + index * 32, 154, [], [], matched));
  const barProof = [[1351, 495], [1353, 599], [1355, 515], [1357, 591]].map(([id, x]) =>
    verifyTacticalBitmap(viewport, stable.bytes, id, x, 189));
  // Against the VM's Imperial Star Destroyer (a0-vm/coruscant-676/05),
  // whose list holds a character and Stormtrooper Regiments like the
  // Empire fixture's: the three regiment cells and the bar, every pixel.
  const contentsA0 = path.join(root, ".artifacts/a0-vm/coruscant-676/05-tf1-selected.png");
  if (ready.faction === "empire" && viewport.width === 640 && viewport.height === 480
    && viewport.device_scale_factor === 1 && fs.existsSync(contentsA0)) {
    const a0 = PNG.sync.read(fs.readFileSync(contentsA0));
    const port = PNG.sync.read(stable.bytes);
    const differ = (x0, y0, x1, y1) => {
      let count = 0;
      for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
        const i = (y * 640 + x) * 4;
        // The icons are colour-matched to the battle's palette as the
        // 8-bit screen does: the Stormtroopers' entry 199, (238, 142, 142)
        // in the bitmap, shows as (194, 158, 149).
        if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[i + c])) count++;
      }
      return count;
    };
    const offA0 = { regiments: differ(527, 154, 619, 186), bar: differ(495, 186, 620, 199) };
    barProof.push({ type: "contents-a0", ...offA0 });
    assert.deepEqual(offA0, { regiments: 0, bar: 0 }, JSON.stringify(offA0));
  }
  // The right arrow scrolls a cell, here the 4 pixels the list overflows:
  // the icons move 4 left and the thumb to 518.
  {
    const scale = Math.min(viewport.width / 640, viewport.height / 480);
    const left = (viewport.width - 640 * scale) / 2;
    const top = (viewport.height - 480 * scale) / 2;
    await page.mouse.click(left + 609 * scale, top + 193 * scale);
    await page.waitForTimeout(300);
    const scrolled = await stableInteractionFrame(page, folder, "contents-scrolled");
    barProof.push(verifyTacticalBitmap(viewport, scrolled, 1355, 518, 189));
    contentProof.push(verifyTacticalBitmap(
      viewport, scrolled, selected.contents[1].icon_id, 523, 154, [], [], matched));
    await page.mouse.click(left + 505 * scale, top + 193 * scale);
    await page.waitForTimeout(300);
    barProof.push(verifyTacticalBitmap(
      viewport, await stableInteractionFrame(page, folder, "contents-scrolled-back"), 1355, 515, 189));
  }

  const destroyed = ready.effects.find(({ resource_base: resourceBase }) => resourceBase === 3360);
  assert.ok(destroyed, "detail fixture omitted the source destroyed sequence");
  assert.deepEqual(destroyed, {
    target: destroyed.target,
    resource_base: 3360,
    resource_id: 3368,
    frame: 8,
    frame_count: 16,
    source_size: [64, 64],
  });
  assert.equal(ready.participants[destroyed.target]?.active, false,
    "destroyed sequence remained attached to an active hull");
  const effectLine = consoleLines.find(({ text }) => text.includes("[tactical_3d] effect_scene"));
  assert.match(effectLine?.text || "",
    new RegExp(`requested=1 rendered=1 .*${destroyed.target + 1}:3368:`),
    "destroyed source frame was not submitted at its retired hull position");
  // A0 TAC-03-C004 (the VM's Battle at Mon Calamari, frame 17, the
  // Alliance's Medium Transport 2 destroyed): a fireball of the 3360..3375
  // family, 36 x 23 pixels at x 165..200, y 292..314, in five colours of
  // the battle palette. The fixture holds frame 8 (3368), A0 a frame not
  // known, so the shapes differ; every colour of A0's fireball must show
  // in the port's, as both index the same palette (5530 + the selector,
  // FUN_005d38d0).
  const explosionA0 = path.join(root, ".artifacts/interface-parity/a0/TAC-03-C004/original.png");
  let explosionProof = null;
  if (viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1
    && fs.existsSync(explosionA0)) {
    const fire = (r, g, b) => r > 150 && b < 120;
    const colours = (png, [x0, y0, x1, y1]) => {
      const found = new Map();
      let box = null;
      for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
        const i = (y * png.width + x) * 4;
        const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
        if (!fire(r, g, b)) continue;
        const key = `${r},${g},${b}`;
        found.set(key, (found.get(key) ?? 0) + 1);
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)]
          : [x, y, x, y];
      }
      return { colours: Object.fromEntries([...found].sort()), box };
    };
    const at = (effectLine.text.match(/:3368:\d+:([\d.]+),([\d.]+)/) || []).slice(1).map(Number);
    assert.equal(at.length, 2, `the destroyed effect's position: ${effectLine.text}`);
    const port = colours(PNG.sync.read(stable.bytes),
      [Math.round(at[0]) - 32, Math.round(at[1]) - 32, Math.round(at[0]) + 32, Math.round(at[1]) + 32]);
    const a0 = colours(PNG.sync.read(fs.readFileSync(explosionA0)), [150, 280, 220, 330]);
    const missing = Object.keys(a0.colours).filter((key) => !(key in port.colours));
    explosionProof = { type: "destroyed-explosion-a0", a0, port, missing };
    assert.deepEqual(a0.box, [165, 292, 200, 314], JSON.stringify(explosionProof));
    assert.ok(Object.keys(a0.colours).length >= 5 && missing.length === 0,
      `the explosion's colours against A0: ${JSON.stringify(explosionProof)}`);
  }

  const participantLine = consoleLines.find(({ text }) =>
    text.includes("[tactical_3d] participant_scene"));
  const positions = parseTacticalScenePositions(participantLine, "escort participant scene");
  const friendlyObjectIds = ready.participants
    .map((participant, index) => ({ participant, objectId: index + 1 }))
    .filter(({ participant }) => participant.kind === "capital-ship"
      && participant.faction === ready.faction && participant.active)
    .map(({ objectId }) => objectId);
  assert.ok(friendlyObjectIds.length >= 2, "escort fixture omitted its friendly target hull");
  // FUN_005d9640's camera can leave a friendly hull outside the battle
  // view (x 16..456, y 28..468) or at its edge; escort the one nearest the
  // view's centre.
  const viewScale = Math.min(viewport.width / 640, viewport.height / 480);
  const viewLeft = (viewport.width - 640 * viewScale) / 2;
  const viewTop = (viewport.height - 480 * viewScale) / 2;
  const insideView = (point) => point
    && point.x >= viewLeft + 16 * viewScale && point.x < viewLeft + 456 * viewScale
    && point.y >= viewTop + 28 * viewScale && point.y < viewTop + 468 * viewScale;
  const viewCentre = { x: viewLeft + 236 * viewScale, y: viewTop + 248 * viewScale };
  const centreDistance = (objectId) => {
    const point = positions.get(objectId);
    return (point.x - viewCentre.x) ** 2 + (point.y - viewCentre.y) ** 2;
  };
  const escortTargetId = friendlyObjectIds.slice(1)
    .filter((objectId) => insideView(positions.get(objectId)))
    .sort((left, right) => centreDistance(left) - centreDistance(right))[0];
  const escortTarget = positions.get(escortTargetId);
  assert.ok(escortTarget, "escort target omitted its source-projected screen position");
  await page.mouse.click(escortTarget.x, escortTarget.y, { button: "right" });
  await page.waitForTimeout(80);
  const assigned = await stableInteractionFrame(page, folder, "escort-assigned");
  const escortLine = consoleLines.find(({ text }) =>
    text.includes("[tactical_escort] event=assigned order=1")
      && text.includes(`target_object_id=${escortTargetId}`));
  assert.match(escortLine?.text || "", /capital_members=1 .*source_projection=true/,
    "direct friendly right-click did not enter source order code 1");
  assert.notEqual(sha256(assigned), sha256(stable.bytes),
    "Escort assignment produced no stable framebuffer transition");
  // FUN_005e4b40: order code 1 keeps "Escort" before the escorted ship's
  // name (the mover's +0x68, as FUN_005e16a0 switches on it).
  const escortOrders = consoleLines.map(({ text }) => text.match(/\[tactical_panel\] orders_line=(.*)$/))
    .filter(Boolean).map((match) => match[1]);
  assert.match(escortOrders.at(-1) ?? "", /^Escort \S/, JSON.stringify(escortOrders));

  const before = PNG.sync.read(stable.bytes);
  const after = PNG.sync.read(assigned);
  const radius = 72 * Math.min(viewport.width / 640, viewport.height / 480);
  let newWhitePixels = 0;
  for (let y = Math.max(0, Math.floor(escortTarget.y - radius));
    y <= Math.min(after.height - 1, Math.ceil(escortTarget.y + radius)); y++) {
    for (let x = Math.max(0, Math.floor(escortTarget.x - radius));
      x <= Math.min(after.width - 1, Math.ceil(escortTarget.x + radius)); x++) {
      const offset = (y * after.width + x) * 4;
      const afterColor = after.data.subarray(offset, offset + 3);
      const beforeColor = before.data.subarray(offset, offset + 3);
      const isWhite = afterColor[0] >= 248 && afterColor[1] >= 248 && afterColor[2] >= 248;
      const wasWhite = beforeColor[0] >= 248 && beforeColor[1] >= 248 && beforeColor[2] >= 248;
      if (isWhite && !wasWhite) newWhitePixels++;
    }
  }
  assert.ok(newWhitePixels >= 8,
    "Escort target did not receive the manual-defined white selection box");

  fs.writeFileSync(path.join(folder, "destroyed-and-contents.png"), stable.bytes);
  return [{
    type: "source-traced-destroyed-capital",
    executable_functions: ["FUN_005d3e90", "FUN_005d41a0"],
    effect: destroyed,
    explosion_a0: explosionProof,
    screenshot_sha256: sha256(stable.bytes),
  }, {
    type: "escort-orders-line",
    orders_line: escortOrders.at(-1),
  }, {
    type: "source-bitmap-selected-hull-contents",
    panel_resource: 1302,
    label: content.label,
    bitmap: contentProof,
    bar: barProof,
  }, {
    type: "source-order-escort",
    executable_functions: ["FUN_005ca6d0", "FUN_005d0af0", "FUN_005cf910"],
    order_code: 1,
    target_object_id: escortTargetId,
    white_box_new_pixels: newWhitePixels,
    screenshot_sha256: sha256(assigned),
  }];
}

async function probeTacticalProjectileFieldPresentation(folder, stable, consoleLines, ready) {
  const expectedProjectiles = [
    { source: 0, target: 3, kind: "laser", longitudinal_scale: 0.5,
      shape_variant: 2, color_selector: 0, elapsed: 0.5, duration: 1, progress: 0.5 },
    { source: 4, target: 1, kind: "turbolaser", longitudinal_scale: 0.65,
      shape_variant: 1, color_selector: 1, elapsed: 0.5, duration: 1, progress: 0.5 },
    { source: 2, target: 5, kind: "ion", longitudinal_scale: 1,
      shape_variant: 3, color_selector: 2, elapsed: 0.5, duration: 2, progress: 0.25 },
  ];
  const expectedFields = [
    {
      target: 1,
      resource_id: 3623,
      frame: 3,
      tractor_sources: 1,
      gravity_sources: 0,
      tractor_source_ids: [3],
      gravity_source_ids: [],
    },
    {
      target: 4,
      resource_id: 3525,
      frame: 5,
      tractor_sources: 1,
      gravity_sources: 1,
      tractor_source_ids: [0],
      gravity_source_ids: [2],
    },
  ];
  assert.deepEqual(ready.projectiles, expectedProjectiles,
    "projectile fixture did not expose the exact source profiles");
  assert.deepEqual(ready.fields, expectedFields,
    "field fixture did not preserve tractor/gravity priority and resources");
  assert.deepEqual(ready.effects, [],
    "projectile/field fixture unexpectedly installed an impact sequence");

  const loadLines = consoleLines.filter(({ text }) =>
    text.includes("[tactical_3d] effect_loaded"));
  assert.equal(loadLines.length, 2,
    "field fixture did not load exactly two source frames");
  const loaded = loadLines.map(({ text }) => {
    const match = text.match(/resource=(\d+)/);
    assert.ok(match, "field load omitted its resource identity");
    assert.match(text,
      /texture_filter=nearest transparent_index=\d+ source=FUN_005d38d0/,
      "field load omitted its transparency, filter, or decoder contract");
    return Number(match[1]);
  }).sort((left, right) => left - right);
  assert.deepEqual(loaded, [3525, 3623]);

  const fieldLines = consoleLines.filter(({ text }) =>
    text.includes("[tactical_3d] field_scene"));
  assert.equal(fieldLines.length, 1,
    "field fixture emitted the wrong number of stable scene states");
  assert.match(fieldLines[0].text,
    /requested=2 rendered=2 .*frame_seconds=0\.1 source_size=128x128 priority=gravity_over_tractor .*source=FUN_005b23e0,FUN_005b24d0,FUN_005b2440,FUN_005b2480,FUN_005d3ac0,FUN_005d3cc0/,
    "field scene omitted its dimensions, timing, priority, or source contract");
  for (const { target, resource_id: resourceId } of expectedFields) {
    assert.match(fieldLines[0].text, new RegExp(`${target + 1}:${resourceId}:\\d+:`),
      `field ${resourceId} was not attached to source object ${target + 1}`);
  }

  const projectileLines = consoleLines.filter(({ text }) =>
    text.includes("[tactical_3d] projectile_scene"));
  assert.equal(projectileLines.length, 1,
    "projectile fixture emitted the wrong number of stable scene states");
  assert.match(projectileLines[0].text,
    /requested=3 rendered=3 .*selectors=0,1,2 durations=1,1,2 interpolation=source_to_live_target geometry=source_mesh raster_compat=device_pixel_floor .*FUN_005ee590,LAB_005eeb90/,
    "projectile scene omitted its shapes, colors, lifecycle, geometry, or interpolation source");
  for (const [index, projectile] of expectedProjectiles.entries()) {
    assert.match(projectileLines[0].text,
      new RegExp(`${index + 2001}:${projectile.shape_variant}:`
        + `${projectile.longitudinal_scale.toFixed(3)}:${projectile.progress.toFixed(3)}:`),
      `projectile ${index} omitted its exact shape, scale, or progress`);
  }
  const projectileCenters = [...projectileLines[0].text.matchAll(
    /(20\d\d):\d+:[\d.]+:[\d.]+:([\d.]+),([\d.]+)/g,
  )].map((match) => ({ object_id: Number(match[1]), x: Number(match[2]), y: Number(match[3]) }));
  assert.equal(projectileCenters.length, expectedProjectiles.length,
    "projectile scene did not expose all framebuffer probe centers");
  const screenshot = PNG.sync.read(stable.bytes);
  const selectorPixelCounts = projectileCenters.map((center, index) => {
    const selector = expectedProjectiles[index].color_selector;
    let matchingPixels = 0;
    for (let y = Math.max(0, Math.round(center.y) - 6);
      y <= Math.min(screenshot.height - 1, Math.round(center.y) + 6); y++) {
      for (let x = Math.max(0, Math.round(center.x) - 6);
        x <= Math.min(screenshot.width - 1, Math.round(center.x) + 6); x++) {
        const offset = (y * screenshot.width + x) * 4;
        const [red, green, blue] = screenshot.data.subarray(offset, offset + 3);
        const matches = selector === 0
          ? red >= 48 && green <= 24 && blue <= 24
          : selector === 1
            ? green >= 48 && red <= 24 && blue <= 24
            : blue >= 48 && red <= 24 && green <= 24;
        if (matches) matchingPixels++;
      }
    }
    assert.ok(matchingPixels > 0,
      `projectile ${center.object_id} produced no selector ${selector} framebuffer pixels`);
    return { object_id: center.object_id, color_selector: selector, matching_pixels: matchingPixels };
  });
  fs.writeFileSync(path.join(folder, "projectiles-and-fields.png"), stable.bytes);
  return [
    {
      type: "source-traced-tactical-projectiles",
      executable_functions: [
        "FUN_005b1ea0", "FUN_005b1f60", "FUN_005b2080", "FUN_005b2c70",
        "FUN_005d3de0", "FUN_005ee590", "LAB_005eeb90",
      ],
      thresholds: { laser: 28.8, turbolaser: 34.666668, ion: 32, torpedo: 12.8 },
      source_vertices: 12,
      shape_variants: [1, 2, 3],
      material_selectors: { alliance: 0, empire: 1, ion: 2 },
      durations: [1, 1, 2],
      raster_compatibility: "one-device-pixel floor for source 0.001-unit faces",
      framebuffer_color_pixels: selectorPixelCounts,
      projectiles: expectedProjectiles,
    },
    {
      type: "source-traced-tactical-fields",
      executable_functions: [
        "FUN_005b23e0", "FUN_005b24d0", "FUN_005b2440", "FUN_005b2480",
        "FUN_005d3ac0", "FUN_005d3cc0",
      ],
      families: { tractor: [3620, 3627], gravity: [3520, 3527] },
      source_size: [128, 128],
      frame_seconds: 0.1,
      priority: "gravity-over-tractor-with-tractor-restoration",
      fields: expectedFields,
    },
  ];
}

function verifyDeathStarLaserMeter(viewport, bytes, [r, g, b]) {
  const screenshot = PNG.sync.read(bytes);
  const rect = { x: 508, y: 9, width: 95, height: 6 };
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const total = Math.floor((rect.width) * scale) * Math.floor((rect.height) * scale);
  const pixels = nativeRectPixelCount(viewport, screenshot, rect,
    (red, green, blue) => red === r && green === g && blue === b);
  assert.ok(pixels >= total * 0.9,
    `Death Star laser meter has ${pixels} of ~${total} pixels in ${r},${g},${b}`);
  return { pixels, expected_rgb: [r, g, b] };
}

function nativeRectPixelCount(viewport, screenshot, rect, matches) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const x0 = Math.ceil(offsetX + rect.x * scale);
  const y0 = Math.ceil(offsetY + rect.y * scale);
  const x1 = Math.floor(offsetX + (rect.x + rect.width) * scale);
  const y1 = Math.floor(offsetY + (rect.y + rect.height) * scale);
  let count = 0;
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const offset = (y * screenshot.width + x) * 4;
      if (matches(...screenshot.data.subarray(offset, offset + 3))) count++;
    }
  }
  return count;
}

function probeTacticalSelectedDamagePresentation(
  viewport, folder, stable, ready, expectedDamage = null,
  evidence = { filename: "selected-damage.png", type: "source-mapped-selected-capital-damage" },
) {
  const selected = ready.selected_ship;
  assert.ok(selected, "selected-damage fixture omitted its selected ship");
  const expected = ready.faction === "alliance"
    ? { class_dat_id: 64, hud_resource: 2001 }
    : { class_dat_id: 128, hud_resource: 2016 };
  assert.equal(selected.faction, ready.faction);
  assert.equal(selected.class_dat_id, expected.class_dat_id);
  assert.equal(selected.hud_resource, expected.hud_resource);
  assert.ok(selected.hull_max > 0 && selected.shield_max > 0);
  assert.equal(selected.hull_current, expectedDamage?.hull
    ?? Math.max(1, Math.trunc(selected.hull_max * 2 / 5)));
  assert.equal(selected.shield_current, expectedDamage?.shield
    ?? Math.max(0, Math.trunc(selected.shield_max / 4)));

  const screenshot = PNG.sync.read(stable.bytes);
  // A0 TAC-03-C006/C005/C003: hull on the left, shield on the right, both
  // 37x10 with pure blue remaining for either faction.
  const shieldPixels = nativeRectPixelCount(
    viewport,
    screenshot,
    { x: 585, y: 109, width: selected.shield_current > 0
      ? 37 * selected.shield_current / selected.shield_max : 37, height: 10 },
    (red, green, blue) => red === 0 && green === 0 && blue === 255,
  );
  const hullPixels = nativeRectPixelCount(
    viewport,
    screenshot,
    { x: 516, y: 109, width: 37 * selected.hull_current / selected.hull_max, height: 10 },
    (red, green, blue) => red === 0 && green === 0 && blue === 255,
  );
  if (selected.shield_current > 0) {
    assert.ok(shieldPixels > 0, "selected ship shield meter produced no exact blue pixels");
  } else {
    assert.equal(shieldPixels, 0, "empty selected ship shield meter retained blue pixels");
  }
  assert.ok(hullPixels > 0, "selected ship hull meter produced no exact blue pixels");

  let panelProof = { status: "non-native-scale", pixels_checked: 0 };
  let portraitProof = { status: "non-native-scale", opaque_pixels_checked: 0 };
  if (viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1) {
    panelProof = verifyTacticalBitmap(viewport, stable.bytes, 1302, 482, 28, [
      { id: 1101, x: 489, y: 38 },
      { id: 1103, x: 614, y: 38 },
    ], [
      { x: 10, y: 8, width: 130, height: 54 }, // name, portrait (26, 17) 96x39, caption to row 60
      { x: 34, y: 81, width: 37, height: 10 },
      { x: 103, y: 81, width: 37, height: 10 },
      { x: 10, y: 103, width: 130, height: 15 },
      { x: 12, y: 183, width: 130, height: 24 },
      { x: 12, y: 210, width: 126, height: 22 },
    ]);
    const portrait = decodeIndexedBmp(fs.readFileSync(path.join(
      root, `data/base/ui/tactical-dll/BMP/${selected.hud_resource}.bmp`,
    )));
    const panel = decodeIndexedBmp(fs.readFileSync(path.join(
      root, "data/base/ui/tactical-dll/BMP/1302.bmp",
    )));
    let opaquePixels = 0;
    let exactOpaquePixels = 0;
    for (let y = 0; y < portrait.height; y++) {
      for (let x = 0; x < portrait.width; x++) {
        const color = portrait.pixel(x, y);
        if (color[0] < 32 && color[1] > 192 && color[2] < 32) continue;
        opaquePixels++;
        const offset = ((45 + y) * screenshot.width + 508 + x) * 4; // A0 TAC-03-C001
        if (Array.from(screenshot.data.subarray(offset, offset + 3))
          .every((channel, index) => channel === color[index])) exactOpaquePixels++;
      }
    }
    assert.ok(opaquePixels > 0 && exactOpaquePixels > opaquePixels / 2,
      "selected ship portrait did not retain a majority of exact source pixels");
    // The portrait's bottom-right lime pixel; the name and caption cover
    // its top row and bottom-left corner.
    const matteOffset = (83 * screenshot.width + 603) * 4;
    assert.deepEqual(
      Array.from(screenshot.data.subarray(matteOffset, matteOffset + 3)),
      panel.pixel(121, 55),
      "selected ship lime matte erased the underlying 1302 display grid",
    );
    portraitProof = {
      status: "source-bitmap-composited",
      resource_id: selected.hud_resource,
      opaque_pixels_checked: opaquePixels,
      exact_opaque_pixels: exactOpaquePixels,
      lime_matte_reveals_panel: true,
    };
  }
  fs.writeFileSync(path.join(folder, evidence.filename), stable.bytes);
  return [{
    type: evidence.type,
    source_registry_function: "FUN_00597610",
    panel_resource: 1302,
    portrait_resource: selected.hud_resource,
    shield: { current: selected.shield_current, maximum: selected.shield_max,
      exact_color_pixels: shieldPixels },
    hull: { current: selected.hull_current, maximum: selected.hull_max,
      exact_color_pixels: hullPixels, faction_color: ready.faction },
    panel_proof: panelProof,
    portrait_proof: portraitProof,
  }];
}

function probeTacticalLiveSubsystemDamagePresentation(viewport, folder, stable, ready) {
  const selected = ready.selected_ship;
  assert.ok(selected, "live subsystem fixture omitted its selected ship");
  const probes = probeTacticalSelectedDamagePresentation(
    viewport,
    folder,
    stable,
    ready,
    { hull: selected.hull_max - 11, shield: 0 },
  );
  const expectedLimits = [4, 4, 4, 4, 2];
  const expectedHits = [1, 2, 3, 4, 2];
  const expectedPercentages = [74, 49, 24, 0, 0];
  const expectedResources = [1204, 1208, 1212, 1216, 1221];
  assert.deepEqual(selected.subsystem_hit_limits, expectedLimits);
  assert.deepEqual(selected.subsystem_damage_hits, expectedHits);
  assert.deepEqual(selected.subsystem_percentages, expectedPercentages);
  assert.deepEqual(selected.subsystem_resources, expectedResources);
  const subsystemProofs = expectedResources.map((resourceId, index) =>
    verifyTacticalBitmap(viewport, stable.bytes, resourceId, 492 + index * 27, 131)); // A0 TAC-03-C001
  fs.writeFileSync(path.join(folder, "live-subsystem-damage.png"), stable.bytes);
  probes.push({
    type: "source-traced-live-subsystem-damage",
    executable_functions: [
      "FUN_005b54d0", "FUN_005b1970", "FUN_005b1770", "FUN_005b0400",
      "FUN_005b0460", "FUN_005b1790", "FUN_005b17f0", "FUN_005b1bc0",
      "FUN_005b1680", "FUN_005b16b0", "FUN_005b16e0", "FUN_005b1710",
      "FUN_005b1740",
    ],
    hit_limits: expectedLimits,
    damage_hits: expectedHits,
    percentages: expectedPercentages,
    resources: expectedResources,
    subsystem_proofs: subsystemProofs,
    rng_contract: "inclusive-0-through-100-range-with-deterministic-session-bridge",
  });
  return probes;
}

function probeTacticalSubsystemRepairMobilityPresentation(
  viewport, folder, stable, ready, consoleLines,
) {
  const selected = ready.selected_ship;
  assert.ok(selected, "subsystem repair fixture omitted its selected ship");
  const probes = probeTacticalSelectedDamagePresentation(
    viewport,
    folder,
    stable,
    ready,
    { hull: selected.hull_max, shield: selected.shield_current },
  );
  const expectedHits = [1, 2, 1, 1, 1];
  const expectedPercentages = [75, 50, 75, 50, 50];
  const expectedResources = [1205, 1209, 1215, 1219, 1224];
  assert.deepEqual(selected.subsystem_damage_hits, expectedHits);
  assert.deepEqual(selected.subsystem_percentages, expectedPercentages);
  assert.deepEqual(selected.subsystem_resources, expectedResources);
  assert.equal(selected.base_engine_power, 100);
  assert.equal(selected.engine_mode_bonus, 1);
  assert.equal(selected.active_tractor_power, 25);
  assert.equal(selected.effective_engine_power, 50.75);
  assert.equal(selected.damage_control, 100);
  assert.equal(ready.subsystem_repairs.length, 1);
  assert.deepEqual(ready.subsystem_repairs[0], {
    ship: ready.fields[0].target,
    kind: "engines",
    hits_before: 2,
    hits_after: 1,
  });
  assert.equal(ready.fields.length, 1);
  assert.equal(ready.fields[0].tractor_sources, 1);
  assert.equal(ready.fields[0].gravity_sources, 0);
  const fieldLine = consoleLines.find(({ text }) =>
    text.includes("[tactical_3d] field_scene"));
  const fieldPosition = fieldLine?.text.match(
    /resources=\d+:\d+:\d+:(-?[\d.]+),(-?[\d.]+)/,
  );
  assert.ok(fieldPosition, "subsystem repair fixture omitted its projected tractor field");
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const halfField = 64 * scale;
  const fieldCenter = { x: Number(fieldPosition[1]), y: Number(fieldPosition[2]) };
  const aperture = {
    x0: offsetX + 16 * scale,
    y0: offsetY + 28 * scale,
    x1: offsetX + 460 * scale,
    y1: offsetY + 467 * scale,
  };
  assert.ok(fieldCenter.x - halfField >= aperture.x0
    && fieldCenter.x + halfField <= aperture.x1
    && fieldCenter.y - halfField >= aperture.y0
    && fieldCenter.y + halfField <= aperture.y1,
  "tractor field source bounds escape the tactical aperture");
  const subsystemProofs = expectedResources.map((resourceId, index) =>
    verifyTacticalBitmap(viewport, stable.bytes, resourceId, 492 + index * 27, 131)); // A0 TAC-03-C001
  fs.writeFileSync(path.join(folder, "subsystem-repair-mobility.png"), stable.bytes);
  probes.push({
    type: "source-traced-subsystem-repair-mobility",
    executable_functions: [
      "FUN_005b0330", "FUN_005b1490", "FUN_005b1ab0", "FUN_005b17f0",
      "FUN_005b16b0", "FUN_005b1790",
    ],
    repair_interval_ms: 50000,
    repair: ready.subsystem_repairs[0],
    damage_hits: expectedHits,
    percentages: expectedPercentages,
    resources: expectedResources,
    base_engine_power: selected.base_engine_power,
    engine_mode_bonus: selected.engine_mode_bonus,
    active_tractor_power: selected.active_tractor_power,
    effective_engine_power: selected.effective_engine_power,
    field_source_bounds_contained: true,
    subsystem_proofs: subsystemProofs,
  });
  return probes;
}

function probeTacticalManeuverMovementPresentation(viewport, folder, stable, ready) {
  const selected = ready.selected_ship;
  assert.ok(selected, "maneuver movement fixture omitted its selected ship");
  const probes = probeTacticalSelectedDamagePresentation(
    viewport,
    folder,
    stable,
    ready,
    { hull: selected.hull_max, shield: selected.shield_max },
  );
  const alliance = ready.faction === "alliance";
  const initialZ = alliance ? -56 : 56;
  const directionZ = alliance ? 1 : -1;
  // The anchor moves 0.95 x 78.75 x 0.25 (0x005eb1a0, DAT_0066d84c); the
  // ship, within one frame's travel and facing its heading, snaps onto it
  // (FUN_005b2f30, DAT_0066c508), so its velocity stays zero.
  const expectedZ = initialZ + directionZ * 0.95 * 78.75 * 0.25;
  assert.equal(selected.base_engine_power, 100);
  assert.equal(selected.maneuver_state_value, 4);
  assert.equal(selected.engine_mode_bonus, 5);
  assert.ok(Math.abs(selected.active_tractor_power) === 0);
  assert.equal(selected.effective_engine_power, 78.75);
  assert.equal(selected.movement_alignment, 1);
  assert.ok(Math.abs(selected.source_forward[0]) === 0
    && Math.abs(selected.source_forward[1]) === 0
    && selected.source_forward[2] === directionZ);
  assert.ok(Math.abs(selected.source_desired_forward[0]) === 0
    && Math.abs(selected.source_desired_forward[1]) === 0
    && selected.source_desired_forward[2] === directionZ);
  assert.ok(Math.abs(selected.source_velocity[0]) === 0
    && Math.abs(selected.source_velocity[1]) === 0
    && Math.abs(selected.source_velocity[2]) === 0);
  assert.ok(Math.abs(selected.source_position[0]) === 0
    && selected.source_position[1] === 0
    && Math.abs(selected.source_position[2] - expectedZ) < 0.0001);
  const selectedParticipant = ready.participants.find((participant) =>
    participant.kind === "capital-ship"
      && participant.faction === ready.faction
      && participant.fleet_roster_index === 0);
  assert.ok(selectedParticipant, "maneuver movement fixture omitted its selected participant");
  assert.deepEqual(selectedParticipant.source_position, selected.source_position,
    "render participant did not consume the integrated source position");
  fs.writeFileSync(path.join(folder, "maneuver-movement.png"), stable.bytes);
  probes.push({
    type: "source-traced-tactical-maneuver-movement",
    executable_functions: [
      "FUN_005ad750", "FUN_005afb70", "FUN_005b17f0", "FUN_005b2f30", "FUN_005cd640",
    ],
    maneuver_state_value: selected.maneuver_state_value,
    engine_mode_bonus: selected.engine_mode_bonus,
    effective_engine_power: selected.effective_engine_power,
    alignment: selected.movement_alignment,
    delta_milliseconds: 250,
    initial_position: [-0, 0, initialZ],
    velocity: selected.source_velocity,
    integrated_position: selected.source_position,
    rendered_participant_position: selectedParticipant.source_position,
  });
  return probes;
}

async function probeTacticalCommandAssignmentPresentation(
  page, viewport, folder, stable, consoleLines, ready,
) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const click = async (x, y) => {
    const target = point(x, y);
    await page.mouse.click(target.x, target.y);
    await page.waitForTimeout(80);
  };
  const capture = async (name) => stableInteractionFrame(page, folder, name);
  const probes = [
    // A0 TAC-05-C012/C015: Maneuvers Tactics (1107) on the left,
    // Missions (1105) on the right.
    {
      type: "maneuvers-button-initial",
      ...verifyTacticalBitmap(viewport, stable.bytes, 1107, 494, 238),
    },
    {
      type: "missions-button-initial",
      ...verifyTacticalBitmap(viewport, stable.bytes, 1105, 562, 238),
    },
  ];

  await click(522, 248);
  const maneuversOpen = await capture("maneuvers-open");
  probes.push({ type: "maneuvers-panel", ...verifyTacticalBitmap(
    viewport, maneuversOpen, 1309, 482, 28, [], [
      { x: 14, y: 14, width: 126, height: 108 },
      { x: 13, y: 129, width: 58, height: 58 },
      { x: 75, y: 144, width: 28, height: 28 },
      { x: 111, y: 132, width: 24, height: 52 },
      { x: 66, y: 202, width: 67, height: 25 },
    ],
  ) });
  probes.push({ type: "stand-off-default", ...verifyTacticalBitmap(
    viewport, maneuversOpen, 1117, 593, 160,
  ) });

  // A0 TAC-05-C013: Hammer held shows its chosen art 1138 and leaves the
  // preview as it was (no maneuver chosen yet); the release chooses it.
  const hammer = point(508, 170);
  await page.mouse.move(hammer.x, hammer.y);
  await page.mouse.down();
  await page.waitForTimeout(80);
  const hammerHeld = await capture("maneuvers-hammer-held");
  probes.push({ type: "hammer-held", ...verifyTacticalBitmap(
    viewport, hammerHeld, 1138, 495, 157,
  ) });
  {
    const open = PNG.sync.read(maneuversOpen);
    const held = PNG.sync.read(hammerHeld);
    let differing = 0;
    for (let y = Math.round(offsetY + 42 * scale); y < Math.round(offsetY + 150 * scale); y++) {
      for (let x = Math.round(offsetX + 496 * scale); x < Math.round(offsetX + 622 * scale); x++) {
        const i = (y * open.width + x) * 4;
        if (open.data[i] !== held.data[i] || open.data[i + 1] !== held.data[i + 1]
          || open.data[i + 2] !== held.data[i + 2]) differing++;
      }
    }
    assert.equal(differing, 0, "holding Hammer changed the maneuver preview");
    probes.push({ type: "hammer-held-preview-unchanged", differing });
  }
  await page.mouse.up();
  await page.waitForTimeout(300);
  await click(604, 185);
  const hammerSurround = await capture("maneuvers-hammer-surround");
  probes.push({ type: "hammer-selected", ...verifyTacticalBitmap(
    viewport, hammerSurround, 1138, 495, 157,
  ) });
  probes.push({ type: "surround-selected", ...verifyTacticalBitmap(
    viewport, hammerSurround, 1118, 593, 160,
  ) });
  probes.push({ type: "hammer-graphic", ...verifyTacticalTransparentOverlay(
    viewport, hammerSurround, 2103, 496, 42,
  ) });
  await click(600, 241);
  const afterCancel = await capture("maneuvers-cancelled");
  probes.push({ type: "cancel-restores-display", ...verifyTacticalBitmap(
    viewport, afterCancel, 1105, 562, 238,
  ) });

  await click(522, 248);
  await click(508, 170);
  await click(604, 185);
  await click(560, 241);
  const maneuverCommitted = await capture("maneuvers-committed");
  probes.push({ type: "maneuver-commit-restores-display", ...verifyTacticalBitmap(
    viewport, maneuverCommitted, 1105, 562, 238,
  ) });

  await click(522, 248);
  await click(538, 170);
  await click(604, 185);
  const anvilStandOff = await capture("maneuvers-anvil-stand-off");
  probes.push({ type: "anvil-selected", ...verifyTacticalBitmap(
    viewport, anvilStandOff, 1140, 525, 157,
  ) });
  probes.push({ type: "stand-off-selected", ...verifyTacticalBitmap(
    viewport, anvilStandOff, 1117, 593, 160,
  ) });
  probes.push({ type: "anvil-graphic", ...verifyTacticalTransparentOverlay(
    viewport, anvilStandOff, 2104, 496, 42,
  ) });
  await click(560, 241);
  await capture("maneuvers-anvil-stand-off-committed");

  // A0 TAC-05-C001 (Left Hook chosen: 1134 at (495, 187), preview 2101)
  // and TAC-05-C004 (Stop chosen: 1142 at (557, 172), preview 2105), each
  // chosen after another maneuver; Cancel leaves both unassigned.
  await click(522, 248);
  await click(509, 201);
  const leftHook = await capture("maneuvers-left-hook");
  probes.push({ type: "left-hook-selected", ...verifyTacticalBitmap(
    viewport, leftHook, 1134, 495, 187,
  ) });
  probes.push({ type: "left-hook-graphic", ...verifyTacticalTransparentOverlay(
    viewport, leftHook, 2101, 496, 42,
  ) });
  await click(571, 186);
  const hold = await capture("maneuvers-hold");
  probes.push({ type: "hold-selected", ...verifyTacticalBitmap(
    viewport, hold, 1142, 557, 172,
  ) });
  probes.push({ type: "left-hook-released", ...verifyTacticalBitmap(
    viewport, hold, 1133, 495, 187,
  ) });
  probes.push({ type: "hold-graphic", ...verifyTacticalTransparentOverlay(
    viewport, hold, 2105, 496, 42,
  ) });
  await click(600, 241);
  await capture("maneuvers-hold-cancelled");

  await click(590, 248);
  const missionsOpen = await capture("missions-open-capital");
  probes.push({ type: "missions-panel", ...verifyTacticalBitmap(
    viewport, missionsOpen, 1308, 482, 28, [], [
      { x: 14, y: 14, width: 124, height: 106 },
      { x: 18, y: 129, width: 114, height: 56 },
      { x: 66, y: 202, width: 67, height: 25 },
    ],
  ) });
  probes.push({ type: "capital-recover-disabled", ...verifyTacticalBitmap(
    viewport, missionsOpen, ready.faction === "alliance" ? 1172 : 1175, 568, 157,
  ) });
  probes.push({ type: "death-star-disabled", ...verifyTacticalBitmap(
    viewport, missionsOpen, 1178, 568, 187,
  ) });
  await click(522, 169);
  const capitalMission = await capture("missions-attack-capital-selected");
  const capitalButton = ready.faction === "alliance" ? 1180 : 1183;
  const capitalGraphic = ready.faction === "alliance" ? 2151 : 2155;
  probes.push({ type: "attack-capital-selected", ...verifyTacticalBitmap(
    viewport, capitalMission, capitalButton, 500, 157,
  ) });
  probes.push({ type: "attack-capital-graphic", ...verifyTacticalTransparentOverlay(
    viewport, capitalMission, capitalGraphic, 496, 42,
  ) });
  // A0 TAC-05-C008 (Alliance): Attack Fighters chosen after Attack Capital
  // Ships is 1192 at (500, 187), the TIE target, and Attack Capital Ships
  // goes back to 1179. Then Attack Capital Ships again, for the commit.
  await click(523, 200);
  const fighterMission = await capture("missions-attack-fighters-selected");
  probes.push({ type: "attack-fighters-selected", ...verifyTacticalBitmap(
    viewport, fighterMission, ready.faction === "alliance" ? 1192 : 1195, 500, 187,
  ) });
  probes.push({ type: "attack-capital-released", ...verifyTacticalBitmap(
    viewport, fighterMission, ready.faction === "alliance" ? 1179 : 1182, 500, 157,
  ) });
  await click(522, 169);
  await click(560, 241);

  await click(359, 10);
  const fighterSelected = await capture("fighter-group-selected");
  probes.push({ type: "fighter-group-selected", ...verifySquadronButton(
    viewport, fighterSelected, 1016, 347, 2,
  ) });
  await click(590, 248);
  const fighterMissions = await capture("missions-open-fighter");
  probes.push({ type: "fighter-recover-enabled", ...verifyTacticalBitmap(
    viewport, fighterMissions, ready.faction === "alliance" ? 1170 : 1173, 568, 157,
  ) });
  await click(590, 169);
  const recoverSelected = await capture("missions-recover-selected");
  probes.push({ type: "recover-selected", ...verifyTacticalBitmap(
    viewport, recoverSelected, ready.faction === "alliance" ? 1171 : 1174, 568, 157,
  ) });
  probes.push({ type: "recover-graphic", ...verifyTacticalTransparentOverlay(
    viewport, recoverSelected, ready.faction === "alliance" ? 2153 : 2157, 496, 42,
  ) });
  await click(560, 241);
  await capture("fighter-recover-committed");
  // SetOrder (0x005ca530) posts the executor (event 0xa) three turns
  // ahead; the no-room voice comes from that executor (FUN_005cf980), so
  // the battle runs until it.
  await click(575, 319);

  const orderLogs = consoleLines
    .filter(({ text }) => text.includes("[tactical_orders]"))
    .map(({ text }) => text);
  assert.ok(orderLogs.some((line) => line.includes("event=cancel")),
    "maneuver cancel did not emit its deterministic transition");
  assert.ok(orderLogs.some((line) =>
    /panel=maneuvers event=commit order=9 tactic=1 capital_members=1 fighter_members=0/.test(line)),
  "maneuver assignment did not commit Hammer and Surround to the selected capital");
  assert.ok(orderLogs.some((line) =>
    /panel=maneuvers event=commit order=10 tactic=2 capital_members=1 fighter_members=0/.test(line)),
  "maneuver assignment did not commit Anvil and Stand Off to the selected capital");
  assert.ok(orderLogs.some((line) =>
    /panel=missions event=commit order=5 tactic=0 capital_members=1 fighter_members=0/.test(line)),
  "capital mission did not commit Attack Capital Ships");
  assert.ok(orderLogs.some((line) =>
    /panel=missions event=commit order=2 tactic=0 capital_members=0 fighter_members=2/.test(line)),
  "fighter mission did not commit Recover");
  const expectedCapacityVoice = ready.faction === "alliance"
    ? { event: "0x8b", wave: 14108, owner: "Alliance" }
    : { event: "0x109", wave: 15112, owner: "Empire" };
  const findCapacityVoice = () => consoleLines.find(({ text }) =>
    text.includes("voice_event=FighterRecoveryCapacity")
      && text.includes(`source_event=${expectedCapacityVoice.event}`));
  let capacityVoice = findCapacityVoice();
  const capacityDeadline = Date.now() + 30000;
  while (!capacityVoice && Date.now() < capacityDeadline) {
    await page.waitForTimeout(100);
    capacityVoice = findCapacityVoice();
  }
  await click(575, 319);
  assert.match(capacityVoice?.text || "",
    new RegExp(`wave=${expectedCapacityVoice.wave} faction=${expectedCapacityVoice.owner} routed=true loaded=true muted=true`),
    "fighter Recover did not route the exact muted no-capacity warning");
  probes.push({
    type: "source-traced-tactical-command-assignment",
    executable_functions: [
      "FUN_005dcc70", "FUN_005dd080", "FUN_005dd140", "FUN_005dd290", "FUN_005dd3a0",
      "FUN_005def00", "FUN_005df110", "FUN_005df4f0", "FUN_005df5f0", "FUN_005df660",
    ],
    order_codes: {
      recover: 2,
      attack_fighters: 4,
      attack_capital_ships: 5,
      attack_death_star: 6,
      left_hook: 7,
      right_hook: 8,
      hammer: 9,
      anvil: 10,
      hold_position: 11,
    },
    tactic_codes: { surround: 1, stand_off: 2 },
    logs: orderLogs,
    capacity_voice_log: capacityVoice.text,
  });
  return probes;
}

function probeTacticalCommandExecutionPresentation(viewport, folder, stable, ready) {
  const selected = ready.selected_ship;
  assert.ok(selected, "command execution fixture omitted its selected capital");
  assert.equal(selected.order_code, 7, "selected capital did not retain Left Hook");
  assert.equal(selected.tactic_code, 1, "selected capital did not retain Surround");
  assert.ok(Array.isArray(selected.source_waypoint)
    && selected.source_waypoint.length === 3
    && selected.source_waypoint.every(Number.isFinite),
  "Left Hook did not create a finite source waypoint");
  const desiredMagnitude = Math.hypot(...selected.source_desired_forward);
  assert.ok(Math.abs(desiredMagnitude - 1) < 0.00001,
    "Left Hook desired direction is not normalized");

  const selectedParticipant = ready.participants.find((participant) =>
    participant.kind === "capital-ship"
      && participant.faction === ready.faction
      && participant.fleet_roster_index === 0);
  assert.ok(selectedParticipant, "command execution fixture omitted its selected participant");
  assert.equal(selectedParticipant.order_code, 7);
  assert.equal(selectedParticipant.tactic_code, 1);
  assert.deepEqual(selectedParticipant.source_waypoint, selected.source_waypoint);
  assert.deepEqual(selectedParticipant.source_desired_forward,
    selected.source_desired_forward);

  const held = ready.participants.find((participant) =>
    participant.kind === "capital-ship"
      && participant.faction === ready.faction
      && participant.order_code === 11);
  assert.ok(held, "command execution fixture omitted its held capital");
  assert.equal(held.source_waypoint, null);
  assert.deepEqual(held.source_desired_forward, [0, 0, 0]);

  const returning = ready.participants.find((participant) =>
    participant.kind === "fighter-group"
      && participant.faction === ready.faction
      && participant.order_code === 2);
  assert.ok(returning, "command execution fixture omitted its recovering fighter group");
  assert.equal(returning.recovery_state_code, 2);
  assert.ok(Number.isSafeInteger(returning.recovery_target)
    && returning.recovery_target >= 0,
  "Recover did not reserve a carrier");
  const carrier = ready.participants[returning.recovery_target];
  assert.ok(carrier?.kind === "capital-ship" && carrier.faction === ready.faction,
    "Recover reserved an incompatible carrier");

  fs.writeFileSync(path.join(folder, "command-execution.png"), stable.bytes);
  return [{
    type: "source-traced-tactical-command-execution",
    executable_functions: [
      "FUN_005ca6d0", "FUN_005cef00", "FUN_005cf1a0", "FUN_005cf410",
      "FUN_005cf680", "FUN_005cf930", "FUN_005cf940", "FUN_005cf980",
      "FUN_005d21b0", "FUN_005cda40", "FUN_005a9530", "FUN_005a8f70",
    ],
    maneuver: {
      order_code: selected.order_code,
      tactic_code: selected.tactic_code,
      waypoint_scale: 0.75,
      angle_radians: Math.PI / 8,
      waypoint: selected.source_waypoint,
      desired_forward: selected.source_desired_forward,
    },
    hold: {
      order_code: held.order_code,
      waypoint: held.source_waypoint,
      desired_forward: held.source_desired_forward,
    },
    recover: {
      order_code: returning.order_code,
      state_code: returning.recovery_state_code,
      carrier: returning.recovery_target,
    },
  }];
}

function probeTacticalCommandProgressionPresentation(
  viewport, folder, stable, consoleLines, ready,
) {
  const selected = ready.selected_ship;
  assert.ok(selected, "command progression fixture omitted its turning capital");
  assert.equal(selected.order_code, 7);
  assert.equal(selected.maneuverability, 4);
  const expectedTurn = 5 * Math.PI / 72;
  const forwardSign = ready.faction === "alliance" ? 1 : -1;
  assert.ok(Math.abs(selected.source_forward[0] - Math.sin(expectedTurn)) < 0.00001,
    "turning capital did not apply the executable-derived X component");
  assert.ok(Math.abs(selected.source_forward[1]) < 0.00001);
  assert.ok(Math.abs(selected.source_forward[2] - forwardSign * Math.cos(expectedTurn)) < 0.00001,
    "turning capital did not apply the executable-derived Z component");
  assert.ok(Math.abs(Math.hypot(...selected.source_forward) - 1) < 0.00001,
    "turning capital direction is not normalized");

  const playerCapitals = ready.participants.filter((participant) =>
    participant.kind === "capital-ship" && participant.faction === ready.faction);
  const arrived = playerCapitals.find((participant) =>
    participant.fleet_roster_index === 1);
  assert.ok(arrived, "command progression fixture omitted its arriving capital");
  assert.equal(arrived.order_code, 0);
  assert.equal(arrived.source_waypoint, null);
  // At its place the ship's heading is its forward (FUN_005b4d40).
  assert.deepEqual(arrived.source_desired_forward, [0, 0, forwardSign]);
  // The point lies within the route's 1.0 of the anchor (0x005eb1a0): the
  // force arrives where it stands.
  const expectedArrivalZ = ready.faction === "alliance"
    ? ready.source_layout.outer_negative_z
    : ready.source_layout.outer_positive_z;
  assert.equal(arrived.source_position[2], expectedArrivalZ);

  const playerFighters = ready.participants.filter((participant) =>
    participant.kind === "fighter-group" && participant.faction === ready.faction);
  assert.equal(playerFighters.length, 2);
  const docking = playerFighters.find((participant) => participant.recovery_state_code === 3);
  const recovered = playerFighters.find((participant) => participant.recovery_state_code === 4);
  assert.ok(docking, "returning fighter did not enter the source docking state");
  assert.equal(docking.active, true);
  assert.equal(docking.order_code, 2);
  assert.equal(docking.recovery_target, ready.faction === "alliance" ? 1 : 4);
  // It starts 2.5 behind the carrier's underside and flies its frame
  // (0x5b8f70) at its speed 5 for 250 ms toward it (0x5c7060, 0x5b0b80).
  assert.equal(Math.abs(docking.source_position[2] - arrived.source_position[2]), 1.25);
  assert.ok(recovered, "docking callback did not enter the source recovered state");
  assert.equal(recovered.active, false);
  assert.equal(recovered.order_code, 0);
  assert.equal(recovered.recovery_target, docking.recovery_target);
  assert.deepEqual(recovered.source_position, arrived.source_position);
  assert.equal(recovered.fighter_group, 1);
  const expectedRecoveryVoice = ready.faction === "alliance"
    ? { event: "0x88", wave: 14105, owner: "Alliance" }
    : { event: "0x106", wave: 15109, owner: "Empire" };
  const recoveryVoice = consoleLines.find(({ text }) =>
    text.includes("voice_event=FighterRecoveryComplete")
      && text.includes(`source_event=${expectedRecoveryVoice.event}`));
  assert.match(recoveryVoice?.text || "",
    new RegExp(`wave=${expectedRecoveryVoice.wave} faction=${expectedRecoveryVoice.owner} routed=true loaded=true muted=true`),
    "fighter recovery did not route the exact muted group-completion voice");

  fs.writeFileSync(path.join(folder, "command-progression.png"), stable.bytes);
  return [{
    type: "source-traced-tactical-command-progression",
    executable_functions: [
      "FUN_005b9c60", "FUN_005ba270", "FUN_005cd2d0", "FUN_005cd460",
      "FUN_005cd640", "FUN_005cf190", "FUN_005cf8f0", "FUN_005cf980",
      "FUN_005b1e10", "FUN_005b84e0", "FUN_005b8630",
    ],
    turn: {
      raw_maneuverability: selected.maneuverability,
      radians: expectedTurn,
      source_forward: selected.source_forward,
    },
    arrival: {
      roster: arrived.fleet_roster_index,
      position: arrived.source_position,
      order_code: arrived.order_code,
      waypoint: arrived.source_waypoint,
    },
    recover: {
      docking_state_code: docking.recovery_state_code,
      recovered_state_code: recovered.recovery_state_code,
      carrier: docking.recovery_target,
      docking_distance: 2,
      strategic_count_preserved: true,
      completion_voice_log: recoveryVoice.text,
    },
  }];
}

function probeTacticalAttackTargetingPresentation(viewport, folder, stable, ready, consoleLines) {
  const playerCapitals = ready.participants.filter((participant) =>
    participant.kind === "capital-ship" && participant.faction === ready.faction);
  const playerFighters = ready.participants.filter((participant) =>
    participant.kind === "fighter-group" && participant.faction === ready.faction);
  const enemyCapitalIndex = ready.participants.findIndex((participant) =>
    participant.kind === "capital-ship" && participant.faction !== ready.faction);
  const enemyFighterIndex = ready.participants
    .filter((participant) => participant.kind === "fighter-group")
    .findIndex((participant) => participant.faction !== ready.faction);
  assert.ok(enemyCapitalIndex >= 0, "attack fixture omitted its hostile capital target");
  assert.ok(enemyFighterIndex >= 0, "attack fixture omitted its hostile fighter target");

  const capitalToCapital = playerCapitals.find((participant) =>
    participant.fleet_roster_index === 0);
  const capitalToFighter = playerCapitals.find((participant) =>
    participant.fleet_roster_index === 1);
  const fighterToCapital = playerFighters.find((participant) =>
    participant.fleet_roster_index === 0);
  const fighterToFighter = playerFighters.find((participant) =>
    participant.fleet_roster_index === 1);
  assert.deepEqual(
    [capitalToCapital?.order_code, capitalToCapital?.attack_target_kind,
      capitalToCapital?.attack_target_index],
    [5, "capital-ship", enemyCapitalIndex],
  );
  assert.deepEqual(
    [capitalToFighter?.order_code, capitalToFighter?.attack_target_kind,
      capitalToFighter?.attack_target_index],
    [4, "fighter-group", enemyFighterIndex],
  );
  assert.deepEqual(
    [fighterToCapital?.order_code, fighterToCapital?.attack_target_kind,
      fighterToCapital?.attack_target_index],
    [5, "capital-ship", enemyCapitalIndex],
  );
  assert.deepEqual(
    [fighterToFighter?.order_code, fighterToFighter?.attack_target_kind,
      fighterToFighter?.attack_target_index],
    [4, "fighter-group", enemyFighterIndex],
  );
  assert.equal(ready.selected_ship?.order_code, 4);
  // FUN_005e4b40: a held target reads "Attacking" and its name (A0
  // TAC-02/03 "Attacking Victory Destroyer 1", "Attacking Swift").
  const ordersLines = consoleLines.map(({ text }) => text.match(/\[tactical_panel\] orders_line=(.*)$/))
    .filter(Boolean).map((match) => match[1]);
  assert.match(ordersLines.at(-1) ?? "", /^Attacking \S/,
    `the targeting ship's order line: ${JSON.stringify(ordersLines)}`);

  fs.writeFileSync(path.join(folder, "attack-targeting.png"), stable.bytes);
  return [{
    type: "attacking-orders-line",
    orders_line: ordersLines.at(-1),
  }, {
    type: "source-traced-tactical-attack-targeting",
    executable_functions: [
      "FUN_005ca6d0", "FUN_005d0b10", "FUN_005d0bb0", "FUN_005c91e0",
      "FUN_005c91c0", "FUN_005f5060", "FUN_005a8fc0",
    ],
    object_type_codes: { "capital-ship": 0, "fighter-group": 1 },
    source_event: "0x36",
    assignments: [capitalToCapital, capitalToFighter, fighterToCapital, fighterToFighter]
      .map((participant) => ({
        owner_kind: participant.kind,
        owner_roster: participant.fleet_roster_index,
        order_code: participant.order_code,
        target_kind: participant.attack_target_kind,
        target_index: participant.attack_target_index,
      })),
  }];
}

// A0 TAC-03-C017 (the VM's Empire game, the Battle at Yavin, day 234):
// Task Force 1 had been "Attacking Swift"; the Alliance withdrew and Swift
// jumped out, and the line went back to the order's word, "Attack Capital
// Ships", over "Tactics: Stand Off", with no target box. FUN_005e4b40
// names a target only while the mover holds one (+0x98); FUN_005cdf50
// drops a ship whose jump has started (state 6) from the target lists.
function probeTacticalAttackTargetLostPresentation(viewport, folder, stable, ready, consoleLines) {
  const enemyCapitals = ready.participants.filter((participant) =>
    participant.kind === "capital-ship" && participant.faction !== ready.faction);
  assert.ok(enemyCapitals.length >= 1 && enemyCapitals.every(({ retreating }) => retreating),
    `every hostile capital jumps out: ${JSON.stringify(enemyCapitals)}`);
  const capital = ready.participants.find((participant) => participant.kind === "capital-ship"
    && participant.faction === ready.faction && participant.fleet_roster_index === 0);
  assert.deepEqual([ready.selected_ship?.order_code, capital?.order_code,
    capital?.attack_target_kind ?? null], [5, 5, null], JSON.stringify(capital));
  const ordersLines = consoleLines.map(({ text }) => text.match(/\[tactical_panel\] orders_line=(.*)$/))
    .filter(Boolean).map((match) => match[1]);
  assert.equal(ordersLines.at(-1), "Attack Capital Ships", JSON.stringify(ordersLines));
  const proof = { type: "attack-target-lost-orders-line", orders_line: ordersLines.at(-1) };
  const a0File = path.join(root, ".artifacts/interface-parity/a0/TAC-03-C017/original.png");
  if (ready.faction === "empire" && viewport.width === 640 && viewport.height === 480
    && viewport.device_scale_factor === 1 && fs.existsSync(a0File)) {
    const a0 = PNG.sync.read(fs.readFileSync(a0File));
    const port = PNG.sync.read(stable.bytes);
    // The order box's two lines: their ink's box in each, within the
    // accepted text deviation (the port's smoothed Liberation Sans against
    // GDI's unsmoothed Arial, 2026-10-24-tactical-panel-text-a0). Ink is
    // lighter than the box's green (0, 128, 0) and grid (84, 95, 84).
    const ink = (png, [x0, y0, x1, y1]) => {
      let box = null;
      for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
        const i = (y * 640 + x) * 4;
        if (Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) < 100) continue;
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)]
          : [x, y, x, y];
      }
      return box;
    };
    const lines = { order: [494, 211, 600, 221], tactics: [494, 222, 600, 231] };
    proof.a0 = {};
    for (const [name, region] of Object.entries(lines)) {
      proof.a0[name] = { a0: ink(a0, region), port: ink(port, region) };
    }
    fs.writeFileSync(path.join(folder, "target-lost-a0.json"), JSON.stringify(proof, null, 2));
    for (const [name, { a0: a, port: p }] of Object.entries(proof.a0)) {
      assert.ok(a && p && Math.abs(a[0] - p[0]) <= 1 && Math.abs(a[1] - p[1]) <= 1
        && Math.abs(a[3] - p[3]) <= 1 && Math.abs((a[2] - a[0]) - (p[2] - p[0])) <= 0.15 * (a[2] - a[0] + 1),
        `${name}: ${JSON.stringify(proof.a0[name])}`);
    }
  }
  fs.writeFileSync(path.join(folder, "attack-target-lost.png"), stable.bytes);
  return [proof];
}

function probeTacticalAttackTargetLifecyclePresentation(viewport, folder, stable, ready) {
  const playerCapitals = ready.participants.filter((participant) =>
    participant.kind === "capital-ship" && participant.faction === ready.faction);
  const playerFighters = ready.participants.filter((participant) =>
    participant.kind === "fighter-group" && participant.faction === ready.faction);
  const enemyCapitalIndices = ready.participants
    .map((participant, index) => ({ participant, index }))
    .filter(({ participant }) =>
      participant.kind === "capital-ship" && participant.faction !== ready.faction);
  const allFighters = ready.participants.filter((participant) =>
    participant.kind === "fighter-group");
  const enemyFighterIndices = allFighters
    .map((participant, index) => ({ participant, index }))
    .filter(({ participant }) => participant.faction !== ready.faction);
  assert.ok(enemyCapitalIndices.length >= 2,
    "target lifecycle fixture omitted its capital replacement");
  assert.ok(enemyFighterIndices.length >= 2,
    "target lifecycle fixture omitted its fighter replacement");

  const retiredCapital = enemyCapitalIndices[0];
  const replacementCapital = enemyCapitalIndices[1];
  const retiredFighter = enemyFighterIndices[0];
  const replacementFighter = enemyFighterIndices[1];
  assert.equal(retiredCapital.participant.retreating, true);
  assert.equal(replacementCapital.participant.retreating, false);
  assert.equal(retiredFighter.participant.recovery_state_code, 4);
  assert.notEqual(replacementFighter.participant.recovery_state_code, 4);

  const capitalToCapital = playerCapitals.find((participant) =>
    participant.fleet_roster_index === 0);
  const capitalToFighter = playerCapitals.find((participant) =>
    participant.fleet_roster_index === 1);
  const fighterToCapital = playerFighters.find((participant) =>
    participant.fleet_roster_index === 0);
  const fighterToFighter = playerFighters.find((participant) =>
    participant.fleet_roster_index === 1);
  assert.deepEqual(
    [capitalToCapital?.order_code, capitalToCapital?.attack_target_kind,
      capitalToCapital?.attack_target_index],
    [5, "capital-ship", replacementCapital.index],
  );
  assert.deepEqual(
    [capitalToFighter?.order_code, capitalToFighter?.attack_target_kind,
      capitalToFighter?.attack_target_index],
    [4, "fighter-group", replacementFighter.index],
  );
  assert.deepEqual(
    [fighterToCapital?.order_code, fighterToCapital?.attack_target_kind,
      fighterToCapital?.attack_target_index],
    [5, "capital-ship", replacementCapital.index],
  );
  assert.deepEqual(
    [fighterToFighter?.order_code, fighterToFighter?.attack_target_kind,
      fighterToFighter?.attack_target_index],
    [4, "fighter-group", replacementFighter.index],
  );

  fs.writeFileSync(path.join(folder, "attack-target-lifecycle.png"), stable.bytes);
  return [{
    type: "source-traced-tactical-attack-target-lifecycle",
    executable_functions: [
      "FUN_005a8c50", "FUN_005a8cc0", "FUN_005a8d30", "FUN_005a8e30",
      "FUN_005a8e80", "FUN_005a8fc0", "FUN_005d0b00", "FUN_005cf910",
      "FUN_005cf920",
    ],
    invalidated: {
      capital_index: retiredCapital.index,
      fighter_index: retiredFighter.index,
    },
    replacements: [capitalToCapital, capitalToFighter, fighterToCapital, fighterToFighter]
      .map((participant) => ({
        owner_kind: participant.kind,
        owner_roster: participant.fleet_roster_index,
        order_code: participant.order_code,
        target_kind: participant.attack_target_kind,
        target_index: participant.attack_target_index,
      })),
  }];
}

async function probeTacticalDeathStarPresentation(
  page, viewport, folder, stable, consoleLines, ready,
) {
  const deathStar = ready.death_star;
  assert.ok(deathStar, "Death Star fixture omitted its manager-owned object");
  assert.deepEqual({
    faction: deathStar.faction,
    is_attacker: deathStar.is_attacker,
    tactical_ordinal: deathStar.tactical_ordinal,
    resource_id: deathStar.resource_id,
    opposing_resource_id: deathStar.opposing_resource_id,
    operational: deathStar.operational,
    hull: deathStar.hull,
    laser_charge: deathStar.laser_charge,
    destroyed: deathStar.destroyed,
    action_committed: deathStar.action_committed,
  }, {
    faction: "empire",
    is_attacker: false,
    tactical_ordinal: 37,
    resource_id: 5010,
    opposing_resource_id: 5020,
    operational: true,
    hull: 100,
    // FUN_005af4e0 stores +0x6c = 100.0 (A0 TAC-05-C017: full at entry).
    laser_charge: 100,
    destroyed: false,
    action_committed: false,
  });
  assert.deepEqual(deathStar.source_position,
    [0, 0, ready.source_layout.outer_positive_z]);

  const effectLoad = consoleLines.find(({ text }) =>
    text.includes("[tactical_3d] effect_loaded resource=5010"));
  assert.ok(effectLoad, "Death Star resource 5010 was not decoded into the tactical renderer");
  // FUN_005d4d10 hangs 5010 as a backdrop decal, not a field object.
  const backdrop = consoleLines.find(({ text }) =>
    text.includes("[tactical_3d] death_star_backdrop resource=5010"));
  assert.ok(backdrop, "Death Star resource 5010 was not submitted as the backdrop decal");
  fs.writeFileSync(path.join(folder, "death-star-overview.png"), stable.bytes);

  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const click = async (x, y) => {
    await page.mouse.click(offsetX + x * scale, offsetY + y * scale);
    await page.waitForTimeout(80);
  };
  const capture = async (name) => stableInteractionFrame(page, folder, name);

  await click(359, 10);
  await click(590, 248);
  const missions = await capture("death-star-missions-open");
  const enabled = ready.faction === "alliance";
  const probes = [{
    type: enabled ? "attack-death-star-enabled" : "attack-death-star-disabled",
    ...verifyTacticalBitmap(viewport, missions, enabled ? 1176 : 1178, 568, 187),
  }];
  // The run's draws (FUN_005d04e0) wait for its state 5 (FUN_005cfec0),
  // after the approach; entering the run consumes nothing.
  assert.equal(ready.tactical_rng_state, 0x5eedba77,
    `${ready.faction} Death Star journey changed its tactical RNG seed before state 5`);
  probes.push({
    type: "source-tactical-rng-entry",
    source: ["FUN_0061a310", "FUN_005a8a70"],
    rng_state: ready.tactical_rng_state,
  });

  await click(590, 199);
  const afterAttack = await capture(enabled
    ? "death-star-mission-selected"
    : "death-star-disabled-probe");
  if (enabled) {
    probes.push({
      type: "attack-death-star-selected",
      ...verifyTacticalBitmap(viewport, afterAttack, 1177, 568, 187),
    });
    probes.push({
      type: "attack-death-star-graphic",
      ...verifyTacticalTransparentOverlay(viewport, afterAttack, 2154, 496, 42),
    });
    await click(560, 241);
  } else {
    assert.equal(sha256(afterAttack), sha256(missions),
      "friendly Death Star accepted the disabled attack control");
  }

  const orderLogs = consoleLines
    .filter(({ text }) => text.includes("[tactical_orders]"))
    .map(({ text }) => text);
  if (enabled) {
    assert.ok(orderLogs.some((line) =>
      /panel=missions event=commit order=6 tactic=0 capital_members=0 fighter_members=1/.test(line)),
    "Alliance fighter group did not commit Attack Death Star");
    // The order's executor (event 0xa, FUN_005ca6d0) runs three turns
    // later: it creates the run object (0x005d0820) and starts the
    // approach (state 2); the timer and FUN_005d04e0's draws wait for
    // state 5 (FUN_005cfec0).
    await click(575, 319);
    const findLaunchLog = () => consoleLines.find(({ text }) =>
      text.includes("[tactical_death_star] trench_run_launch status=approach fighter_members=1")
        && text.includes("group=0")
        && text.includes("remaining_ms=0"));
    let launchLog = findLaunchLog();
    const launchDeadline = Date.now() + 30000;
    while (!launchLog && Date.now() < launchDeadline) {
      await page.waitForTimeout(100);
      launchLog = findLaunchLog();
    }
    assert.ok(launchLog,
    "Alliance fighter group did not enter the production trench-run lifecycle");
    const launchRngState = Number(launchLog.text.match(/rng_state=(\d+)/)?.[1]);
    const launchWeaponDraws = Number(launchLog.text.match(/weapon_draws=(\d+)/)?.[1]);
    assert.ok(Number.isInteger(launchWeaponDraws),
      "trench-run launch log lost its weapon draw count");
    // Until then only the units' weapon turns draw, once each
    // (FUN_005b3a40, FUN_005a8a70).
    assert.equal(launchRngState,
      advanceTacticalRng(ready.tactical_rng_state, launchWeaponDraws - ready.weapon_draws),
      "the trench-run order drew from the shared tactical RNG before state 5");
  } else {
    assert.ok(!orderLogs.some((line) => /order=6/.test(line)),
      "Imperial fighter group committed an attack against its friendly Death Star");
  }
  const expectedEntryVoice = enabled
    ? {
        name: "TrenchRunStarted", event: "0x139", wave: 15160, faction: "Alliance",
      }
    : {
        name: "DeathStarUnderAttack", event: "0x115", wave: 15124, faction: "Empire",
      };
  const findEntryVoice = () => consoleLines.find(({ text }) =>
    text.includes(`voice_event=${expectedEntryVoice.name}`)
      && text.includes(`source_event=${expectedEntryVoice.event}`)
      && text.includes(`wave=${expectedEntryVoice.wave}`)
      && text.includes(`faction=${expectedEntryVoice.faction}`)
      && text.includes("routed=true")
      && text.includes("loaded=true")
      && text.includes("muted=true"));
  // The voice line can trail the commit by a frame or two.
  let entryVoiceLog = findEntryVoice();
  for (let attempt = 0; attempt < 30 && !entryVoiceLog; attempt += 1) {
    await page.waitForTimeout(100);
    entryVoiceLog = findEntryVoice();
  }
  assert.ok(entryVoiceLog,
    `Death Star entry voice did not route ${JSON.stringify(expectedEntryVoice)}`);
  probes.push({
    type: "death-star-attack-entry-voice",
    source: "FUN_005d0340",
    ...expectedEntryVoice,
    routed: true,
    loaded: true,
    muted: true,
  });
  if (enabled) {
    // FUN_005c6610 flies the approach (the battle is still running from
    // the order's executor); its arrival posts state 5, whose turn starts
    // the 120 s run and its follow-up line.
    const stateFive = "[tactical_death_star] trench_run_state=5 group=0 remaining_ms=120000";
    const stateFiveDeadline = Date.now() + 120000;
    while (!consoleLines.some(({ text }) => text.includes(stateFive))
      && Date.now() < stateFiveDeadline) {
      await page.waitForTimeout(100);
    }
    assert.ok(consoleLines.some(({ text }) => text.includes(stateFive)),
      "the trench run's approach did not reach state 5");
    await click(575, 319);
    let committedVoice;
    for (let attempt = 0; attempt < 30 && !committedVoice; attempt += 1) {
      committedVoice = consoleLines.find(({ text }) =>
        text.includes("voice_event=TrenchRunCommitted")
          && text.includes("source_event=0x13a")
          && text.includes("wave=15161")
          && text.includes("faction=Alliance")
          && text.includes("routed=true")
          && text.includes("loaded=true")
          && text.includes("muted=true"));
      if (!committedVoice) await page.waitForTimeout(100);
    }
    assert.ok(committedVoice, "timed trench-run follow-up voice did not route");
    probes.push({
      type: "trench-run-timed-state-entry",
      source: "FUN_005cfec0",
      group: 0,
      remaining_ms: 120000,
      source_event: "0x13a",
      wave: 15161,
      routed: true,
      loaded: true,
      muted: true,
    });
  }
  probes.push({
    type: "source-traced-tactical-death-star-manager",
    executable_functions: [
      "FUN_005ab0e0", "FUN_005ab650", "FUN_005ba420", "FUN_005ba7f0", "FUN_005df110",
    ],
    object_id: 3001,
    manager: deathStar,
    attack_enabled: enabled,
    order_logs: orderLogs,
  });
  return probes;
}

async function probeTacticalDeathStarLaserJourney(
  page, viewport, folder, stable, consoleLines, ready,
) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const probes = [{
    type: "source-tactical-star-surface",
    ...verifyTacticalBackdrop(viewport, stable.bytes, ready.palette_resource_id),
  }];
  assert.ok(consoleLines.some(({ text }) =>
    text.includes("[tactical_3d] backdrop_loaded resource=5030 dimensions=440x438")),
  "resource 5030 was not decoded as the tactical star surface");
  assert.ok(consoleLines.some(({ text }) =>
    text.includes("[tactical_3d] backdrop_scene resource=5030 dimensions=440x438")),
  "resource 5030 was not submitted behind the tactical scene");

  const button = point(623, 12);
  if (ready.faction === "alliance") {
    await page.mouse.click(button.x, button.y);
    await page.waitForTimeout(80);
    const rejected = await stableInteractionFrame(page, folder, "enemy-death-star-control-absent");
    assert.equal(sha256(rejected), sha256(stable.bytes),
      "Alliance player activated the enemy Death Star control region");
    assert.ok(!consoleLines.some(({ text }) => text.includes("superlaser_targeting status=armed")),
      "Alliance player armed the enemy Death Star superlaser");
    probes.push({
      type: "enemy-death-star-control-absent",
      faction_gate: "empire-owner-only",
      unchanged_sha256: sha256(rejected),
    });
    return probes;
  }

  probes.push({
    type: "death-star-laser-gauge-ready",
    ...verifyTacticalBitmap(viewport, stable.bytes, 1024, 474, 0, [], [
      { x: 0, y: 0, width: 166, height: 1 },
      { x: 0, y: 0, width: 1, height: 25 },
      { x: 163, y: 0, width: 3, height: 25 },
      { x: 34, y: 9, width: 95, height: 6 },
      { x: 137, y: 2, width: 21, height: 20 },
    ]),
  }, {
    type: "death-star-laser-ready",
    ...verifyTacticalBitmap(viewport, stable.bytes, 1021, 611, 2),
  }, {
    // FUN_005c4ed0 meter 95x6 at (508, 9): a full charge is pure green
    // (A0 TAC-05-C017).
    type: "death-star-laser-meter-full",
    ...verifyDeathStarLaserMeter(viewport, stable.bytes, [0, 255, 0]),
  });

  await page.mouse.move(button.x, button.y);
  await page.mouse.down({ button: "left" });
  await page.waitForTimeout(80);
  const held = await stableInteractionFrame(page, folder, "death-star-laser-held");
  probes.push({
    type: "death-star-laser-held",
    ...verifyTacticalBitmap(viewport, held, 1022, 611, 2),
  });
  await page.mouse.up({ button: "left" });
  await page.waitForTimeout(80);
  const armed = await stableInteractionFrame(page, folder, "death-star-laser-armed");
  probes.push({
    type: "death-star-laser-armed",
    ...verifyTacticalBitmap(viewport, armed, 1022, 611, 2),
  });
  assert.ok(consoleLines.some(({ text }) =>
    text.includes("[tactical_death_star] superlaser_targeting status=armed input=right_click")),
  "Death Star fire control did not arm right-click targeting");

  const participantLine = consoleLines.find(({ text }) =>
    text.includes("[tactical_3d] participant_scene"));
  const positions = parseTacticalScenePositions(participantLine, "Death Star target scene");
  const enemyIndex = ready.participants.findIndex(({ kind, faction }) =>
    kind === "capital-ship" && faction === "alliance");
  assert.ok(enemyIndex >= 0, "Death Star journey lacks an Alliance capital target");
  const targetObjectId = enemyIndex + 1;
  const target = positions.get(targetObjectId);
  assert.ok(target, `Death Star target object ${targetObjectId} lacks a projected position`);
  await page.mouse.click(target.x, target.y, { button: "right" });
  await page.waitForTimeout(80);
  const committed = await stableInteractionFrame(page, folder, "death-star-laser-committed");
  probes.push({
    type: "death-star-laser-committed",
    target_object_id: targetObjectId,
    // A0 TAC-05-C018: a committed shot shows 1023 and an empty meter.
    ...verifyTacticalBitmap(viewport, committed, 1023, 611, 2),
  }, {
    type: "death-star-laser-meter-empty",
    ...verifyDeathStarLaserMeter(viewport, committed, [0, 0, 255]),
  });
  assert.ok(consoleLines.some(({ text }) =>
    text.includes(`[tactical_death_star] superlaser_committed target_object_id=${targetObjectId} charge=0`)),
  "Death Star right-click target did not commit the superlaser shot");

  const pause = point(575, 319);
  await page.mouse.click(pause.x, pause.y);
  // 0x005ba720 stamps the beam event 0x44 three 1000 ms turns ahead
  // (+0x8ac, +0x8a8); committed in turn 0, it runs at turn 4.
  const firedLine = `[tactical_death_star] superlaser_fired target_object_id=${targetObjectId}`;
  const firedDeadline = Date.now() + 8000;
  while (!consoleLines.some(({ text }) => text.includes(firedLine)) && Date.now() < firedDeadline) {
    await page.waitForTimeout(10);
  }
  assert.ok(consoleLines.some(({ text }) => text.includes(firedLine)),
    "Death Star beam event did not run at its turn");
  // 0x005da830 grows the beam for 1 s (DAT_0066d4bc), then it ends. From
  // the Death Star at -2e x it enters the first Imperial view after ~0.85 s.
  // Its timer only advances in unpaused frames, so the probe pauses near
  // that time and inspects still frames, resuming a frame or two at a time
  // until the beam shows, instead of racing a moving frame.
  const drawnLine = `[tactical_3d] death_star_beam target_object_id=${targetObjectId}`;
  const drawnDeadline = Date.now() + 4000;
  while (!consoleLines.some(({ text }) => text.includes(drawnLine)) && Date.now() < drawnDeadline) {
    await page.waitForTimeout(10);
  }
  await page.waitForTimeout(700);
  await page.mouse.click(pause.x, pause.y);
  await page.waitForTimeout(80);
  let beam;
  let beamVisibility;
  for (let attempt = 0; attempt < 16 && !beamVisibility; attempt += 1) {
    beam = await stableInteractionFrame(page, folder, "death-star-superlaser-beam");
    try {
      beamVisibility = verifyTacticalGreenBeam(viewport, committed, beam);
    } catch (error) {
      if (!(error instanceof assert.AssertionError)) throw error;
      await page.mouse.click(pause.x, pause.y);
      await page.waitForTimeout(16);
      await page.mouse.click(pause.x, pause.y);
      await page.waitForTimeout(80);
    }
  }
  assert.ok(beamVisibility, "Death Star beam produced no visible green aperture pixels");
  await page.mouse.click(pause.x, pause.y);
  assert.ok(consoleLines.some(({ text }) =>
    text.includes(`[tactical_3d] death_star_beam target_object_id=${targetObjectId} grow_seconds=1 visible=true`)
      && text.includes("source=FUN_005da150,0x005da830")),
  "Death Star beam was not drawn from the backdrop decal to its target");
  await page.waitForTimeout(1300);
  assert.ok(consoleLines.some(({ text }) =>
    text.includes(`[tactical_death_star] superlaser_resolved target_object_id=${targetObjectId}`)),
  "Death Star beam did not resolve against its assigned target");
  await page.mouse.click(pause.x, pause.y);
  await page.waitForTimeout(80);
  const resolved = await stableInteractionFrame(page, folder, "death-star-superlaser-resolved");
  probes.push({
    type: "death-star-superlaser-resolution",
    target_object_id: targetObjectId,
    beam_frame_sha256: sha256(beam),
    beam_visibility: beamVisibility,
    resolved_frame_sha256: sha256(resolved),
    control: verifyTacticalBitmap(viewport, resolved, 1023, 611, 2),
    executable_functions: ["FUN_005ba5e0", "FUN_005ba7f0", "FUN_005c4ed0", "FUN_005d4d10"],
  });
  return probes;
}

// The tactical stream after `count` draws (FUN_0061a310: MSVC rand).
function advanceTacticalRng(state, count) {
  let next = state >>> 0;
  for (let draw = 0; draw < count; draw += 1) {
    next = (Math.imul(next, 0x343fd) + 0x269ec3) >>> 0;
  }
  return next;
}

function probeTacticalTrenchRunOutcome(folder, stable, consoleLines, ready, expected) {
  const movie = expected === "success" ? "201.webm" : "202.webm";
  const expectedChatter = expected === "success"
    ? [0x11e, 0x11f, 0x120, 0x128, 0x121, 0x122, 0x129]
    : [0x11e, 0x123, 0x124, 0x125, 0x12a, 0x12b, 0x126, 0x12c, 0x127];
  assert.equal(ready.trench_run_outcome, expected);
  assert.equal(ready.trench_run_remaining_ms, null,
    "resolved trench run retained an active source timer");
  assert.equal(ready.trench_run_group, 0,
    "resolved trench run lost its RGBY source group");
  assert.notEqual(ready.tactical_rng_state, 0x5eedba77,
    "native trench-run producer did not consume the tactical RNG stream");
  const route = consoleLines.find(({ text }) =>
    text.includes(`[cutscene] opened path=assets/references/ref-videos/${movie}`));
  assert.ok(route, `trench-run ${expected} did not route through ${movie}`);
  const observedChatter = consoleLines
    .filter(({ text }) => text.includes("voice_event=TrenchRunChatter"))
    .map(({ text }) => {
      const match = text.match(/source_event=0x([0-9a-f]+) wave=(\d+)/);
      assert.ok(match, `malformed trench chatter log: ${text}`);
      return {
        source_event: Number.parseInt(match[1], 16),
        wave: Number(match[2]),
      };
    });
  assert.deepEqual(
    observedChatter,
    expectedChatter.map((sourceEvent) => ({
      source_event: sourceEvent,
      wave: 15133 + sourceEvent - 0x11e,
    })),
    `trench-run ${expected} did not preserve the recovered chatter order`,
  );
  fs.writeFileSync(path.join(folder, `trench-run-${expected}-return.png`), stable.bytes);
  return [{
    type: `trench-run-${expected}-route`,
    result_state: expected === "success" ? 6 : 7,
    movie,
    returned_to: "tactical-combat",
    native_producer: ["FUN_005cfec0", "FUN_005d04e0", "FUN_0061a310"],
    fighter_group: ready.trench_run_group,
    rng_state: ready.tactical_rng_state,
    ordered_chatter: observedChatter,
    log: route.text,
  }];
}

async function probeTacticalBattleResultsPresentation(
  page, viewport, folder, stable, consoleLines, faction,
) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const alliance = faction === "alliance";
  // A0 TAC-07-C001 / TAC-07-C002: the window opens at (62, 49) over the
  // Alliance command center and at (125, 52) over the Empire's; the
  // positions below were measured with it at (85, 74).
  const [dx, dy] = alliance ? [-23, -25] : [40, -22];
  const resources = alliance ? {
    frame: 10335, close: 10370, tabs: [10729, 10731, 10733, 10792],
    summary: 10759, detail: 10761, fleet: 10765,
  } : {
    frame: 10336, close: 10376, tabs: [10739, 10741, 10743, 10521],
    summary: 10760, detail: 10761, fleet: 10766,
  };
  const tabX = (alliance ? 523.5 : 533) + dx;
  const tabY = (alliance ? [182.5, 236.5, 290.5, 344.5] : [183.5, 242.5, 301.5, 360.5]).map((y) => y + dy);
  // A0 TAC-01-C006, TAC-01-C013 and battle-result-coruscant: the keyed
  // frame lies over the scene (its arcs at the pane's corners), and the
  // texts over both. Only the text boxes and the rail are skipped.
  const frameIgnored = [
    { x: 12, y: 17, width: 400, height: 43 },
    { x: 12, y: 215, width: 400, height: 94 },
    { x: 412, y: 0, width: 58, height: 330 },
  ];
  const frameSource = decodeIndexedBmp(fs.readFileSync(path.join(
    root, `data/base/ui/strategy-dll/BMP/${resources.frame}.bmp`,
  )));
  const frameCovers = (x, y) => {
    const [r, g, b] = frameSource.pixel(x + 12, y + 13);
    return !(r < 32 && g < 32 && b > 192);
  };
  const probes = [{
    type: "battle-results-frame",
    ...verifyStrategyBitmap(viewport, stable.bytes, resources.frame, 85 + dx, 74 + dy, frameIgnored),
  }, {
    type: "battle-results-summary",
    ...verifyStrategyBitmap(viewport, stable.bytes, resources.summary, 97 + dx, 87 + dy, [
      { x: 0, y: 4, width: 400, height: 43 },
      { x: 0, y: 202, width: 400, height: 94 },
    ], false, frameCovers),
  }, {
    type: "battle-results-close-rest",
    ...verifyStrategyBitmap(
      viewport, stable.bytes, resources.close, (alliance ? 508 : 511) + dx, (alliance ? 99 : 91) + dy,
    ),
  }, {
    type: "battle-results-summary-selected",
    ...verifyStrategyBitmap(
      viewport, stable.bytes, resources.tabs[0], (alliance ? 503 : 511) + dx, (alliance ? 162 : 163) + dy,
    ),
  }];

  const clickTab = async (index, name) => {
    const location = point(tabX, tabY[index]);
    await page.mouse.click(location.x, location.y);
    return stableInteractionFrame(page, folder, name);
  };

  let capture = await clickTab(1, "results-alliance-forces");
  probes.push({
    type: "battle-results-alliance-tab",
    ...verifyStrategyBitmap(
      viewport, capture, resources.tabs[1], (alliance ? 503 : 511) + dx, (alliance ? 216 : 222) + dy,
    ),
  }, {
    type: "battle-results-alliance-detail",
    ...verifyStrategyBitmap(viewport, capture, resources.detail, 97 + dx, 87 + dy, [
      { x: 0, y: 0, width: 400, height: 100 },
      // The list (window 37..384); font 6 empty labels reach x 356.
      { x: 0, y: 100, width: 372, height: 196 },
    ], false, frameCovers),
  });

  // FUN_0046f360: the category bar at (130, 59), four 49 by 41 buttons;
  // Capital Ships opens on the two-column page (STRATEGY 10761), Personnel
  // on the three-column one (10762).
  for (let category = 0; category < 4; category++) {
    const location = point(85 + dx + 130 + category * 49 + 24, 74 + dy + 59 + 20);
    await page.mouse.click(location.x, location.y);
    capture = await stableInteractionFrame(page, folder, `results-category-${category + 1}`);
    probes.push({
      type: "battle-results-force-category",
      category: ["capital-ships", "fighters", "troops", "personnel"][category],
      screenshot_sha256: sha256(capture),
    });
    if (category === 1) {
      // FUN_0046f360 -> FUN_0060a490: four operational squadrons (280
      // pixels) outgrow the 166-high list, so its bar shows at the list's
      // right edge, window (372, 136), with the RCDATA 10653 art clipped
      // to 12 wide. The thumb's top cap starts at the track's top (145).
      const clipped = [{ x: 12, y: 0, width: 1, height: 13 }];
      probes.push({
        type: "battle-results-scroll-bar",
        up_arrow: verifyStrategyBitmap(viewport, capture, 10685, 85 + dx + 372, 74 + dy + 136, clipped),
        down_arrow: verifyStrategyBitmap(viewport, capture, 10687, 85 + dx + 372, 74 + dy + 293, clipped),
        thumb_top: verifyStrategyBitmap(viewport, capture, 10689, 85 + dx + 372, 74 + dy + 145, clipped),
      });
      // A click on the down arrow steps the list's 10 pixels
      // (FUN_00607ea0's step): the range is 280 + 5 - 166 = 119 and the
      // 86-high thumb travels 62, so its top moves to 145 + 5.
      const arrow = point(85 + dx + 378, 74 + dy + 297);
      await page.mouse.click(arrow.x, arrow.y);
      const stepped = await stableInteractionFrame(page, folder, "results-fighters-scrolled");
      probes.push({
        type: "battle-results-scroll-step",
        thumb_top: verifyStrategyBitmap(viewport, stepped, 10689, 85 + dx + 372, 74 + dy + 150, clipped),
      });
    }
    // A0 TAC-07-C007 (the Alliance's Battle at Mon Calamari, the window at
    // (62, 49)): the first destroyed regiment's picture, the Alliance Army
    // Regiment's mini on 11514, at window (223, 146), 122 by 50. The
    // fixture's third one-regiment hull dies with its regiment.
    const troopsA0 = path.join(root, ".artifacts/interface-parity/a0/TAC-07-C007/original.png");
    if (category === 2 && alliance && viewport.width === 640 && viewport.height === 480
      && viewport.device_scale_factor === 1 && fs.existsSync(troopsA0)) {
      const a0 = PNG.sync.read(fs.readFileSync(troopsA0));
      const port = PNG.sync.read(capture);
      const [left, top] = [85 + dx, 74 + dy];
      let differing = 0;
      for (let y = 146; y < 196; y++) {
        for (let x = 223; x < 345; x++) {
          const i = ((49 + y) * 640 + 62 + x) * 4;
          const j = ((top + y) * 640 + left + x) * 4;
          if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[j + c])) differing++;
        }
      }
      probes.push({ type: "battle-results-destroyed-regiment-a0", differing });
      assert.equal(differing, 0, `the destroyed regiment's picture differs by ${differing}`);
    }
    if (category === 3) {
      probes.push({
        type: "battle-results-personnel-detail",
        ...verifyStrategyBitmap(viewport, capture, 10762, 97 + dx, 87 + dy, [
          { x: 0, y: 0, width: 400, height: 100 },
          // The list (window 37..384); font 6 empty labels reach x 356.
          { x: 0, y: 100, width: 372, height: 196 },
        ], false, frameCovers),
      });
    }
  }

  capture = await clickTab(2, "results-imperial-forces");
  probes.push({
    type: "battle-results-imperial-tab",
    ...verifyStrategyBitmap(
      viewport, capture, resources.tabs[2], (alliance ? 503 : 511) + dx, (alliance ? 270 : 281) + dy,
    ),
  });

  capture = await clickTab(3, "results-go-directly-to");
  probes.push({
    type: "battle-results-destination-tab",
    ...verifyStrategyBitmap(
      viewport, capture, resources.tabs[3], (alliance ? 503 : 511) + dx, (alliance ? 324 : 340) + dy,
    ),
  }, {
    type: "battle-results-destination-background",
    // FUN_0046f360: the message line at (24, 40) and the instruction at
    // (24, 130), 328 wide; the buttons at (29, 182) and (229, 182); their
    // labels at y 278 (window coordinates; the art sits at (12, 13)).
    ...verifyStrategyBitmap(viewport, capture, 10763, 97 + dx, 87 + dy, [
      // The title, font 0xb from window row 22 (A0 TAC-07-C013).
      { x: 0, y: 4, width: 400, height: 22 },
      { x: 0, y: 24, width: 400, height: 70 },
      { x: 0, y: 114, width: 400, height: 55 },
      { x: 17, y: 169, width: 169, height: 96 },
      { x: 217, y: 169, width: 169, height: 96 },
      { x: 0, y: 265, width: 400, height: 22 },
    ], false, frameCovers),
  }, {
    type: "battle-results-system-destination",
    ...verifyStrategyBitmap(viewport, capture, 10764, 114 + dx, 256 + dy, [], true),
  }, {
    type: "battle-results-fleet-destination",
    ...verifyStrategyBitmap(viewport, capture, resources.fleet, 314 + dx, 256 + dy, [], true),
  });
  // The destination buttons against A0 battle-clean/02 (the Alliance's
  // Battle at Yavin, the window at (62, 49)): keyed on black, the page's
  // stars and flare show through; every pixel of each button but the
  // label rows (the port's labels' text is the accepted deviation).
  const destinationA0 = path.join(root, ".artifacts/a0-vm/battle-clean/02-display-withdrawn-umgul.png");
  if (alliance && viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1
    && fs.existsSync(destinationA0)) {
    const a0 = PNG.sync.read(fs.readFileSync(destinationA0));
    const port = PNG.sync.read(capture);
    // The port's window corner, (85, 74) as measured, moved by (dx, dy).
    const [left, top] = [85 + dx, 74 + dy];
    const differ = (bx) => {
      let count = 0;
      for (let y = 182; y < 182 + 96; y++) {
        for (let x = bx; x < bx + 169; x++) {
          const i = ((49 + y) * 640 + 62 + x) * 4;
          const j = ((top + y) * 640 + left + x) * 4;
          if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[j + c])) count++;
        }
      }
      return count;
    };
    const buttonsA0 = { system: differ(29), fleet: differ(229) };
    probes.push({ type: "battle-results-destinations-a0", ...buttonsA0 });
    assert.deepEqual(buttonsA0, { system: 0, fleet: 0 }, JSON.stringify(buttonsA0));
  }

  // The page's texts are solid in the viewer's colour (FUN_0044f860: 0xff
  // or 0xff00): A0 TAC-07-C012 draws 2,162 pure (255, 0, 0) pixels in its
  // instruction lines. A 10 px raster scaled up blurred them to at most 191.
  if (viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1) {
    const port = PNG.sync.read(capture);
    const pure = alliance ? [255, 0, 0] : [0, 255, 0];
    const [left, top] = [85 + dx, 74 + dy];
    let solid = 0;
    for (let y = top + 130; y < top + 175; y++) {
      for (let x = left + 24; x < left + 352; x++) {
        const i = (y * 640 + x) * 4;
        if (port.data[i] === pure[0] && port.data[i + 1] === pure[1] && port.data[i + 2] === pure[2]) solid++;
      }
    }
    probes.push({ type: "battle-results-instruction-solid", solid });
    assert.ok(solid >= 200, `the instruction lines hold ${solid} solid pixels`);
  }

  const destination = alliance ? point(198.5 + dx, 304 + dy) : point(398.5 + dx, 304 + dy);
  await page.mouse.click(destination.x, destination.y);
  await page.waitForTimeout(200);
  const routed = await stableInteractionFrame(
    page, folder, alliance ? "results-route-system" : "results-route-fleet",
  );
  assert.notEqual(sha256(capture), sha256(routed), `${faction}: result destination did not route`);
  const route = alliance ? "system" : "fleet";
  assert.ok(consoleLines.some(({ text }) =>
    text.includes(`[tactical-results] destination=${route}`)), `${faction}: missing route log`);
  if (route === "fleet") {
    // The Fleet destination opens the battle fleet's Fleet window with the
    // fleet chosen (FUN_00429440, as the Fleet Finder does), its roster the
    // battle's survivors: the fixture keeps two of its three hulls.
    const opened = consoleLines.map(({ text }) =>
      text.match(/\[tactical-results\] fleet_window system=(.*) ships=(\d+) selected=fleet/))
      .filter(Boolean);
    assert.equal(opened.length, 1, `${faction}: the Fleet window did not open on the fleet`);
    assert.equal(Number(opened[0][2]), 2, `${faction}: the fleet's roster after the battle`);
  }
  const persistence = consoleLines.find(({ text }) =>
    text.includes("[tactical_results] strategic_persistence applied=true"));
  assert.ok(persistence, `${faction}: strategic result was not persisted before routing`);
  // FUN_0046f360 +0x18c: the viewer's win plays track 12 / 14, MDATA.308
  // (Alliance) or MDATA.314 (Empire); both fixtures are the viewer's win.
  const summaryScore = consoleLines.filter(({ text }) =>
    text.includes("[audio] context=battle_window track=original mdata=")).at(-1);
  assert.ok(summaryScore, `${faction}: the Battle Summary started no score`);
  assert.match(summaryScore.text, new RegExp(`mdata=${alliance ? 308 : 314} loaded=true`));
  // The post-battle bombardment and landing this once checked were the
  // port's invention, removed in 6c9220a5 (the original never bombards or
  // lands after a battle, ghidra/notes/bombardment-order.md), with their
  // "[post_battle]" log.
  assert.match(persistence.text, /attacker_capitals=3->2/,
    `${faction}: destroyed attacker capital was not removed from strategic state`);
  assert.match(persistence.text, /defender_capitals=3->2/,
    `${faction}: destroyed defender capital was not removed from strategic state`);
  // Five squadrons a side (four operational, so the Fighters list
  // scrolls); one is destroyed.
  assert.match(persistence.text, /attacker_fighters=5->4/,
    `${faction}: destroyed attacker fighter was not removed from strategic state`);
  assert.match(persistence.text, /defender_fighters=5->4/,
    `${faction}: destroyed defender fighter was not removed from strategic state`);
  probes.push({ type: "battle-results-route", destination: route,
    screenshot_sha256: sha256(routed) });
  probes.push({ type: "strategic-result-persistence", log: persistence.text });

  for (const category of ["Capital Ships", "Fighters", "Troops", "Personnel"]) {
    assert.ok(consoleLines.some(({ text }) => text.includes(`category=${category}`)),
      `${faction}: missing ${category} result category transition`);
  }
  return probes;
}

// Retreat on the Battle Alert (0x67, FUN_00450540 -> FUN_00487830: answer
// 1 with the retreat word) at the viewer's own system, which the enemy
// reached last, by one hull against four. The abstract battle lets it
// leave at once (FUN_005d19a0: its strength under 0.33 of the enemy's),
// and the report reads the viewer's withdrawal (FUN_0049b4d0: 0x7003,
// 0x7008, 0x700d) over its withdrawal scene (FUN_0046f360 +0x54).
// A0: message-rail/retreat-result-commenor (Alliance) and
// battle-alert-empire/04-result (Empire).
async function probeTacticalBattleAlertRetreat(page, viewport, folder, consoleLines, ready, faction) {
  assert.equal(ready.battle_alert_open, true, `${faction}: Battle Alert was bypassed`);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const alliance = faction === "alliance";
  const native = viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1;
  // The alert at (62, 49) / (125, 52), as probeTacticalBattleAlertEntry.
  const [dx, dy] = alliance ? [-3, -5] : [60, -2];
  const before = await stableInteractionFrame(page, folder, "battle-alert-retreat-before");
  const probes = [{
    type: "battle-alert-retreat-enabled",
    ...verifyStrategyBitmap(viewport, before, alliance ? 10974 : 10971, 77 + dx, 350 + dy),
  }];
  const retreat = point(144 + dx, 363.5 + dy);
  await page.mouse.move(retreat.x, retreat.y);
  await page.mouse.down({ button: "left" });
  await page.waitForTimeout(100);
  await page.mouse.up({ button: "left" });
  await page.waitForTimeout(300);
  await page.mouse.move(2, 2);
  const results = await stableInteractionFrame(page, folder, "battle-alert-retreat-results");
  assert.ok(consoleLines.some(({ text }) =>
    text.includes("[battle_alert] command=retreat") && text.includes("status=simulated")),
  `${faction}: Retreat did not route the battle`);
  assert.ok(!consoleLines.some(({ text }) => text.includes("[battle_alert] command=take_command")),
    `${faction}: Retreat entered the tactical battle`);
  const summary = consoleLines.map(({ text }) =>
    text.match(/\[tactical-results\] summary lines=(\[.*\])/)).filter(Boolean);
  assert.equal(summary.length, 1, `${faction}: no single Battle Summary report`);
  const lines = JSON.parse(summary[0][1]);
  const [viewer, enemy] = alliance ? ["Alliance", "Imperial"] : ["Imperial", "Alliance"];
  // A0's words: "The Alliance fleet is defeated." / "Commenor is now under
  // blockade by Imperial forces. The Alliance fleet has withdrawn to
  // Bortras." and "The Imperial fleet is defeated." / "Tralus is now under
  // blockade by Alliance forces. The Imperial fleet has withdrawn to
  // Coruscant."; the names are the fixture's.
  assert.equal(lines.length, 2, JSON.stringify(lines));
  assert.equal(lines[0], `The ${viewer} fleet is defeated. `);
  const body = lines[1].match(new RegExp(
    `^(.+) is now under blockade by ${enemy} forces\\. The ${viewer} fleet has withdrawn to (.+)\\.$`));
  assert.ok(body, `${faction}: the report's body ${JSON.stringify(lines[1])}`);
  assert.equal(body[1], ready.system, `${faction}: the report names another system`);
  assert.notEqual(body[2], ready.system, `${faction}: the fleet withdrew to its own battle`);
  probes.push({ type: "battle-alert-retreat-report", lines });

  // The window's own withdrawal scene, 10757 / 10758, at A0's place, and
  // against A0's pixels: the scene under the keyed frame, less the text
  // boxes (the port's text is the accepted deviation) and A0's pointer.
  const [rx, ry] = alliance ? [-23, -25] : [40, -22];
  const frame = alliance ? 10335 : 10336;
  const scene = alliance ? 10757 : 10758;
  const frameSource = decodeIndexedBmp(fs.readFileSync(path.join(
    root, `data/base/ui/strategy-dll/BMP/${frame}.bmp`,
  )));
  const frameCovers = (x, y) => {
    const [r, g, b] = frameSource.pixel(x + 12, y + 13);
    return !(r < 32 && g < 32 && b > 192);
  };
  const textBoxes = [
    { x: 0, y: 4, width: 400, height: 43 },
    { x: 0, y: 202, width: 400, height: 94 },
  ];
  probes.push({
    type: "battle-alert-retreat-scene",
    ...verifyStrategyBitmap(viewport, results, scene, 97 + rx, 87 + ry, textBoxes, false, frameCovers),
  });
  // A0's pointer (screen coordinates) is skipped. TAC-07-C004 is the
  // Alliance's own Withdraw From Battle at Coruscant (tactical), which
  // ends in the same report state; its pointer is off the scene.
  const captures = alliance ? [
    [".artifacts/a0-vm/message-rail/retreat-result-commenor.png", { x: 140, y: 356, width: 20, height: 24 }],
    [".artifacts/interface-parity/a0/TAC-07-C004/original.png", { x: 0, y: 0, width: 0, height: 0 }],
  ] : [
    [".artifacts/a0-vm/battle-alert-empire/04-result.png", { x: 336, y: 359, width: 24, height: 24 }],
  ];
  for (const [file, pointer] of captures) {
    const a0Path = path.join(root, file);
    if (!native || !fs.existsSync(a0Path)) continue;
    const a0 = PNG.sync.read(fs.readFileSync(a0Path));
    const port = PNG.sync.read(results);
    const [left, top] = [97 + rx, 87 + ry];
    let checked = 0;
    let differ = 0;
    for (let y = 0; y < 310; y++) {
      for (let x = 0; x < 400; x++) {
        if (textBoxes.some((rect) => x >= rect.x && y >= rect.y
          && x < rect.x + rect.width && y < rect.y + rect.height)) continue;
        if (frameCovers(x, y)) continue;
        if (left + x >= pointer.x && top + y >= pointer.y
          && left + x < pointer.x + pointer.width && top + y < pointer.y + pointer.height) continue;
        const i = ((top + y) * 640 + left + x) * 4;
        checked++;
        if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[i + c])) differ++;
      }
    }
    probes.push({ type: "battle-alert-retreat-scene-a0", a0: file, checked, differ });
    assert.equal(differ, 0, `${faction}: the withdrawal scene differs from A0 ${file} in ${differ} of ${checked} pixels`);
  }
  // Retreat's results, like Simulate's, never enter the tactical scene.
  probes.push({ type: "battle-alert-simulate-route", route: "command=retreat",
    screenshot_sha256: sha256(results) });
  return probes;
}

// A0 TAC-04-C014 (the Alliance's Battle at Imperial Coruscant, paused):
// the four navigation sets on (manual p. 144), Red Group selected, then F1,
// "Camera Follows Task Force 1" (manual p. 66; FUN_0059b730 case 0x70,
// FUN_005c2990 mode 0xb builds the chase camera FUN_005d9a70). The probe
// compares the frame outside the battle view and outside text with A0, and
// measures the navigation pyramids the chase view shows in both.
async function probeTacticalCoruscantChase(page, viewport, folder, consoleLines, ready, faction) {
  assert.equal(ready.system, "Coruscant", `${faction}: the battle is not at Coruscant`);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const at = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const click = async (x, y) => {
    const p = at(x, y);
    await page.mouse.move(p.x, p.y);
    await page.mouse.down({ button: "left" });
    await page.waitForTimeout(80);
    await page.mouse.up({ button: "left" });
    await page.waitForTimeout(300);
  };
  const own = ready.participants.filter(({ faction: side }) => side === faction);
  const ships = own.filter(({ kind }) => kind === "capital-ship");
  const groups = own.filter(({ kind, active }) => kind === "fighter-group" && active);
  if (faction === "alliance") {
    // A0's Fleet 2: one Alliance Escort Carrier, an X-wing and a Y-wing
    // squadron (A0 TAC-02-C002, TAC-02-C003).
    assert.deepEqual(ships.map(({ name }) => name.replace(/ \d+$/, "")), ["Alliance Escort Carrier"],
      `${faction}: the fleet is not A0's carrier`);
    assert.equal(groups.length, 2, `${faction}: the fleet does not field two squadrons`);
  }
  // Navigation Point Set 1 to 4 (1026..1029 at x 485, 524, 563, 602).
  for (const x of [485, 524, 563, 602]) await click(x + 13, 285);
  // A0 TAC-04-C001: the same battle's free camera with the four sets on.
  const restAt = at(300, 465);
  await page.mouse.move(restAt.x, restAt.y);
  await page.waitForTimeout(300);
  const navigationSets = await stableInteractionFrame(page, folder, "coruscant-navigation-sets");
  // Red Group: the first squadron button (347, 2; A0 TAC-02-C002).
  if (groups.length > 0) await click(359, 10);
  // A0 then right-clicks its own carrier: Red Group escorts it (manual
  // p. 148, A0 TAC-05-C011). The carrier stands where the scene drew it.
  const scene = consoleLines.find(({ text }) => text.includes("[tactical_3d] participant_scene"));
  const carrierAt = scene?.text.match(/screen_positions=1:([\d.]+),([\d.]+)/);
  if (faction === "alliance" && carrierAt) {
    // The scene logs its projections in canvas pixels, already letterboxed.
    const p = { x: Number(carrierAt[1]), y: Number(carrierAt[2]) };
    await page.mouse.move(p.x, p.y);
    await page.mouse.down({ button: "right" });
    await page.waitForTimeout(80);
    await page.mouse.up({ button: "right" });
    await page.waitForTimeout(300);
  }
  await page.keyboard.press("F1");
  await page.waitForTimeout(300);
  // A0's pointer rests at the bottom of the battle view.
  const rest = at(300, 465);
  await page.mouse.move(rest.x, rest.y);
  await page.waitForTimeout(500);
  const chase = await stableInteractionFrame(page, folder, "coruscant-chase-f1");
  const follow = consoleLines.find(({ text }) =>
    text.includes("[tactical_camera] command=follow kind=task_force group=1"));
  assert.ok(follow, `${faction}: F1 did not follow Task Force 1`);
  if (faction === "alliance") {
    assert.ok(consoleLines.some(({ text }) =>
      /\[tactical_escort\] event=assigned .*target_object_id=1 capital_members=0 fighter_members=1/.test(text)),
    `${faction}: Red Group did not escort the carrier`);
  }
  const probe = { type: "coruscant-chase", follow_log: follow.text,
    ships: ships.map(({ name }) => name), squadrons: groups.length };
  const native = viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1;
  const a0Path = path.join(root, ".artifacts/interface-parity/a0/TAC-04-C014/original.png");
  if (!native || faction !== "alliance") return [probe];
  assert.ok(fs.existsSync(a0Path), `A0 TAC-04-C014 is missing at ${a0Path}`);
  const a0 = PNG.sync.read(fs.readFileSync(a0Path));
  const port = PNG.sync.read(chase);
  // The retained-mode view (tactical_scene_viewport) and its frame, the
  // panel's text lines, and A0's pointer.
  const view = { x: 17, y: 30, width: 440, height: 438 };
  const textBoxes = [
    { x: 66, y: 3, width: 12, height: 13 }, // the task-force button's "1"
    { x: 490, y: 34, width: 145, height: 40 }, // Red Group, its orders, Tactics
    { x: 500, y: 172, width: 110, height: 16 }, // "X-wings: 1 12/12"
  ];
  const pointer = { x: 288, y: 452, width: 32, height: 28 };
  const inside = (rect, x, y) => x >= rect.x && y >= rect.y
    && x < rect.x + rect.width && y < rect.y + rect.height;
  const regions = {
    group_bar: { x: 0, y: 0, width: 468, height: 24 },
    right_panel: { x: 468, y: 0, width: 172, height: 480 },
  };
  probe.outside_view = {};
  const others = [];
  for (const [name, rect] of Object.entries(regions)) {
    let checked = 0;
    let differ = 0;
    for (let y = rect.y; y < rect.y + rect.height; y++) {
      for (let x = rect.x; x < rect.x + rect.width; x++) {
        if (inside(view, x, y) || inside(pointer, x, y)
          || textBoxes.some((box) => inside(box, x, y))) continue;
        const i = (y * 640 + x) * 4;
        checked++;
        if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[i + c])) {
          differ++;
          const pair = `${[...a0.data.subarray(i, i + 3)]}>${[...port.data.subarray(i, i + 3)]}`;
          // One palette entry reads (194, 158, 149) in A0 and (238, 142, 142)
          // in the port, in the ship icon and the X-wing portrait.
          if (pair !== "194,158,149>238,142,142") others.push({ x, y, pair });
        }
      }
    }
    probe.outside_view[name] = { checked, differ };
  }
  probe.outside_view.other_than_one_palette_entry = others.slice(0, 20);
  assert.equal(others.length, 0,
    `${faction}: the frame outside the view differs from A0 TAC-04-C014 ${JSON.stringify(probe.outside_view)}`);
  // The navigation pyramids: 4-connected components (at least 20 px) of
  // pure red and of grey-white inside the view, outside "Battle Paused",
  // with their centroids.
  const pyramids = (image) => {
    const pixel = (x, y) => {
      const k = (y * 640 + x) * 4;
      return [image.data[k], image.data[k + 1], image.data[k + 2]];
    };
    const tests = {
      red: ([r, g, b]) => r >= 120 && g < 50 && b < 50,
      white: ([r, g, b]) => r > 150 && Math.abs(r - g) < 12 && Math.abs(r - b) < 12,
    };
    const label = { x: 17, y: 30, width: 110, height: 20 };
    const components = (test) => {
      const seen = new Uint8Array(640 * 480);
      const found = [];
      for (let y = view.y; y < view.y + view.height; y++) {
        for (let x = view.x; x < view.x + view.width; x++) {
          const key = y * 640 + x;
          if (seen[key] || inside(label, x, y) || !test(pixel(x, y))) continue;
          const stack = [key];
          seen[key] = 1;
          let area = 0;
          let sumX = 0;
          let sumY = 0;
          while (stack.length) {
            const k = stack.pop();
            const [cx, cy] = [k % 640, Math.floor(k / 640)];
            area++;
            sumX += cx;
            sumY += cy;
            for (const [nx, ny] of [[cx + 1, cy], [cx - 1, cy], [cx, cy + 1], [cx, cy - 1]]) {
              const nk = ny * 640 + nx;
              if (!inside(view, nx, ny) || seen[nk] || inside(label, nx, ny) || !test(pixel(nx, ny))) continue;
              seen[nk] = 1;
              stack.push(nk);
            }
          }
          if (area >= 20) {
            found.push({ area, centroid: [Math.round(sumX / area * 10) / 10, Math.round(sumY / area * 10) / 10] });
          }
        }
      }
      return found.sort((a, b) => b.area - a.area);
    };
    return Object.fromEntries(Object.entries(tests).map(([name, test]) => [name, components(test)]));
  };
  // Each A0 pyramid of at least `minimumArea` px against the port's nearest pyramid
  // of its colour. The port draws the pyramids about 1.2 times A0's size
  // (A0 TAC-04-C001, the free camera, below), so areas are not compared
  // and a centroid may sit a few pixels off: tolerance 8 px.
  const match = (a0Image, portImage, tolerance, minimumArea) => {
    const a0Pyramids = pyramids(a0Image);
    const portPyramids = pyramids(portImage);
    const pairs = [];
    for (const colour of ["red", "white"]) {
      for (const { area, centroid } of a0Pyramids[colour].filter(({ area }) => area >= minimumArea)) {
        const nearest = portPyramids[colour].reduce((best, candidate) => {
          const distance = Math.hypot(candidate.centroid[0] - centroid[0], candidate.centroid[1] - centroid[1]);
          return !best || distance < best.distance ? { distance, port: candidate } : best;
        }, null);
        pairs.push({ colour, a0: centroid, a0_area: area, port: nearest?.port.centroid ?? null,
          port_area: nearest?.port.area ?? null,
          distance: nearest ? Math.round(nearest.distance * 10) / 10 : null });
      }
    }
    const worst = Math.max(...pairs.map(({ distance }) => distance ?? Infinity));
    return { tolerance, minimum_area: minimumArea, compared: pairs.length, worst, pairs };
  };
  // The free camera with the four sets on, before F1 (A0 TAC-04-C001).
  const c001 = path.join(root, ".artifacts/interface-parity/a0/TAC-04-C001/original.png");
  assert.ok(fs.existsSync(c001), `A0 TAC-04-C001 is missing at ${c001}`);
  probe.free_camera_pyramids = match(PNG.sync.read(fs.readFileSync(c001)), PNG.sync.read(navigationSets), 2, 20);
  // The chase: coarse, 40 px. The port's camera sits 0.8 times as far
  // from the hull as A0's (README 2026-11-12-coruscant-battle-fixture), so
  // this confirms the chase's view, not its exact distance.
  probe.chase_pyramids = match(a0, port, 40, 100);
  for (const [name, result] of [["free camera (A0 TAC-04-C001)", probe.free_camera_pyramids],
    ["chase (A0 TAC-04-C014)", probe.chase_pyramids]]) {
    assert.ok(result.compared > 0 && result.worst <= result.tolerance,
      `${faction}: the ${name} nav pyramids are off A0's ${JSON.stringify(result)}`);
  }
  return [probe];
}

// A0 TAC-01-C008 (the Empire's Battle at Umgul, paused): Rotate Left 16
// times (the camera sphere's left control, 1049; FUN_005d9640 yaws the
// camera about +Y) shows the holocube's (+x, -y, +z) corner at (106, 298)
// and Umgul's planet decal (FUN_005d4d10: e wide at (2e, r, r)) at the top
// right. The probe measures both in the port and in A0.
async function probeTacticalUmgulBackdrop(page, viewport, folder, ready, faction) {
  assert.equal(ready.system, "Umgul", `${faction}: the battle is not at Umgul`);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const at = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const left = at(506, 402);
  for (let press = 0; press < 16; press++) {
    await page.mouse.move(left.x, left.y);
    await page.mouse.down({ button: "left" });
    await page.waitForTimeout(80);
    await page.mouse.up({ button: "left" });
    await page.waitForTimeout(300);
  }
  // A0's cursor rests at the bottom of the battle view.
  const rest = at(310, 470);
  await page.mouse.move(rest.x, rest.y);
  await page.waitForTimeout(500);
  const turned = await stableInteractionFrame(page, folder, "umgul-rotated-left-16");
  const native = viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1;
  if (!native) return [{ type: "umgul-backdrop", presses: 16 }];
  const measure = (image) => {
    const pixel = (x, y) => {
      const k = (y * 640 + x) * 4;
      return [image.data[k], image.data[k + 1], image.data[k + 2]];
    };
    const blue = ([r, g, b]) => b > 150 && r < 80 && g < 80;
    const bright = (c) => c[0] + c[1] + c[2] > 200 && !blue(c);
    // The corner: the first row, from the top, where the holocube's lines
    // run three blue pixels wide near A0's corner.
    let corner = null;
    for (let y = 270; y < 330 && !corner; y++) {
      for (let x = 80; x < 140; x++) {
        if ([0, 1, 2].every((i) => blue(pixel(x + i, y)))) {
          corner = { x, y };
          break;
        }
      }
    }
    const rowEdge = (y) => {
      for (let x = 200; x < 454; x++) {
        if ([0, 1, 2, 3].every((i) => bright(pixel(x + i, y)))) return x;
      }
      return null;
    };
    const columnEdge = (x) => {
      for (let y = 200; y > 31; y--) {
        if ([0, 1, 2, 3].every((i) => bright(pixel(x, y - i)))) return y;
      }
      return null;
    };
    return {
      corner,
      planet_left: { y30: rowEdge(30), y40: rowEdge(40), y60: rowEdge(60) },
      planet_bottom: { x420: columnEdge(420), x450: columnEdge(450) },
    };
  };
  const port = measure(PNG.sync.read(turned));
  const probe = { type: "umgul-backdrop", presses: 16, port };
  const a0Path = path.join(root, ".artifacts/interface-parity/a0/TAC-01-C008/original.png");
  if (faction === "empire" && fs.existsSync(a0Path)) {
    probe.a0 = measure(PNG.sync.read(fs.readFileSync(a0Path)));
    // The planet stands at the top right in both.
    assert.ok(port.planet_left.y40 !== null && port.planet_left.y40 > 300,
      `${faction}: no planet at the top right ${JSON.stringify(port)}`);
    // Its frame's offsets come from a stream seeded by the clock second
    // (FUN_005d4d10, FUN_005f5750). The fixture pins a second whose draws
    // are A0's (UMGUL_A0_BACKDROP_CLOCK), so every edge lies within 2
    // pixels of A0's. With the frame at the centre of its range they were
    // up to 14 pixels off.
    const edges = [["planet_left", "y30"], ["planet_left", "y40"], ["planet_left", "y60"],
      ["planet_bottom", "x420"], ["planet_bottom", "x450"]];
    probe.planet_edge_misses = Object.fromEntries(edges.map(([edge, at]) =>
      [`${edge}.${at}`, port[edge][at] === null || probe.a0[edge][at] === null
        ? null : port[edge][at] - probe.a0[edge][at]]));
    assert.ok(Object.values(probe.planet_edge_misses).every((miss) => miss !== null && Math.abs(miss) <= 2),
      `${faction}: the planet's edges are off A0's ${JSON.stringify(probe.planet_edge_misses)}`);
    // The corner within 1 pixel: the 3-wide row's first pixel falls on
    // either side of the corner as the GL and D3D line rules differ.
    assert.ok(port.corner && Math.abs(port.corner.x - probe.a0.corner.x) <= 1
      && Math.abs(port.corner.y - probe.a0.corner.y) <= 1,
    `${faction}: the holocube corner ${JSON.stringify(port.corner)} is not near A0's`);
    // The lines through the retained-mode viewport (17, 30, 440, 438)
    // (tactical_scene_viewport): at least 90% of the port's blue line
    // pixels lie on A0's, and every one within a pixel of A0's. With the
    // aperture-wide projection, 120 of 1,153 did.
    const lines = (image) => {
      const set = new Set();
      for (let y = 30; y < 468; y++) {
        for (let x = 17; x < 457; x++) {
          const k = (y * 640 + x) * 4;
          if (image.data[k + 2] > 150 && image.data[k] < 60 && image.data[k + 1] < 60) set.add(y * 640 + x);
        }
      }
      return set;
    };
    const a0Lines = lines(PNG.sync.read(fs.readFileSync(a0Path)));
    const portLines = [...lines(PNG.sync.read(turned))];
    const onA0 = portLines.filter((key) => a0Lines.has(key)).length;
    const nearA0 = portLines.filter((key) => [-641, -640, -639, -1, 0, 1, 639, 640, 641]
      .some((step) => a0Lines.has(key + step))).length;
    probe.lines = { a0: a0Lines.size, port: portLines.length, on_a0: onA0, within_one_pixel: nearA0 };
    assert.ok(portLines.length > 0 && onA0 >= 0.9 * portLines.length && nearA0 === portLines.length,
      `${faction}: the holocube's lines are off A0's ${JSON.stringify(probe.lines)}`);
  }
  return [probe];
}

// A0 TAC-03-C012 (Wine): the Alliance's paused battle at Mon Calamari, Task
// Force 1 (one Medium Transport) selected, the pointer at (562, 255) on the
// capital-ship panel's Missions button. Its tip (TEXTTACT, the shared
// win_tooltip box) stands at the pointer plus (0, 21): (562, 276), 52 by
// 19. The probe selects Task Force 1 (72, 10), rests the pointer at A0's
// place and compares the box. Only the Alliance has an A0 capture; the
// Empire's panel has the same layout, so its box is held to the same rule.
async function probeTacticalCapitalPanelTip(page, viewport, folder, faction) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const at = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const taskForce = at(72, 10);
  await page.mouse.move(taskForce.x, taskForce.y);
  await page.mouse.down({ button: "left" });
  await page.waitForTimeout(100);
  await page.mouse.up({ button: "left" });
  await page.waitForTimeout(300);
  const away = at(320, 300);
  await page.mouse.move(away.x, away.y, { steps: 4 });
  await page.waitForTimeout(300);
  const pointer = [562, 255];
  const missions = at(...pointer);
  await page.mouse.move(missions.x, missions.y, { steps: 4 });
  await page.waitForTimeout(1000);
  const shown = await stableInteractionFrame(page, folder, "capital-panel-missions-tip");
  const native = viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1;
  if (!native) return [{ type: "capital-panel-missions-tip", pointer }];
  // win_tooltip.rs FILL and BORDER, the colours A0 shows.
  const fill = [255, 251, 240];
  const border = [128, 128, 128];
  const tipBox = (image, [px, py]) => {
    const same = (x, y, colour) => {
      const k = (y * image.width + x) * 4;
      return colour.every((value, index) => image.data[k + index] === value);
    };
    for (let y = py; y < Math.min(image.height - 1, py + 40); y++) {
      for (let x = Math.max(0, px - 220); x < Math.min(image.width - 1, px + 160); x++) {
        if (!same(x, y, border) || !same(x + 1, y + 1, fill)) continue;
        let y1 = y + 1;
        while (y1 < image.height && same(x, y1, border)) y1 += 1;
        let x1 = x + 1;
        while (x1 < image.width && same(x1, y1 - 1, border)) x1 += 1;
        return { x, y, width: x1 - x, height: y1 - y };
      }
    }
    return null;
  };
  const port = tipBox(PNG.sync.read(shown), pointer);
  const a0Path = path.join(root, ".artifacts/interface-parity/a0/TAC-03-C012/original.png");
  const probe = { type: "capital-panel-missions-tip", pointer, port };
  assert.ok(port, `${faction}: no tip below the Missions button`);
  if (fs.existsSync(a0Path)) {
    const a0 = tipBox(PNG.sync.read(fs.readFileSync(a0Path)), pointer);
    probe.a0 = a0;
    assert.deepEqual([port.x, port.y, port.height], [a0.x, a0.y, a0.height],
      `${faction}: the Missions tip ${JSON.stringify(port)} is not at A0's ${JSON.stringify(a0)}`);
    // The text's width is the accepted Liberation Sans deviation.
    assert.ok(Math.abs(port.width - a0.width) <= Math.ceil(a0.width * 0.12),
      `${faction}: the Missions tip is ${port.width} wide, A0's ${a0.width}`);
    if (port.width === a0.width) {
      // The frame, pixel for pixel; A0's pointer covers one top pixel.
      const portImage = PNG.sync.read(shown);
      const a0Image = PNG.sync.read(fs.readFileSync(a0Path));
      let checked = 0;
      let differ = 0;
      for (let y = a0.y; y < a0.y + a0.height; y++) {
        for (let x = a0.x; x < a0.x + a0.width; x++) {
          if (y !== a0.y && y !== a0.y + a0.height - 1 && x !== a0.x && x !== a0.x + a0.width - 1) continue;
          const k = (y * 640 + x) * 4;
          checked += 1;
          if ([0, 1, 2].some((i) => portImage.data[k + i] !== a0Image.data[k + i])) differ += 1;
        }
      }
      probe.frame = { checked, differ };
      assert.ok(differ <= 1, `${faction}: the tip's frame differs from A0's ${JSON.stringify(probe.frame)}`);
    }
  }
  return [probe];
}

// A0 TAC-01-C003's battle at Imperial Mon Calamari (one Medium Transport
// against one Victory Destroyer) played until the Alliance's last hull
// dies: the battle ends by itself (A0 TAC-01-C013), and the Battle
// Summary's system button (0xcb, FUN_00470ea0 -> FUN_004f3220 ->
// FUN_00429ce0) opens the system's sector window (A0 TAC-07-C014, the
// Sluis sector window after the battle).
async function probeTacticalBattleLastShipLost(page, viewport, folder, consoleLines, ready, faction) {
  assert.equal(ready.battle_alert_open, false, `${faction}: the Battle Alert is still open`);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const click = async (x, y) => {
    const at = point(x, y);
    await page.mouse.move(at.x, at.y);
    await page.mouse.down({ button: "left" });
    await page.waitForTimeout(100);
    await page.mouse.up({ button: "left" });
    await page.waitForTimeout(300);
  };
  const alliance = faction === "alliance";
  const native = viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1;
  // The pause control resumes the battle; the cursor then leaves the view.
  await click(575, 319);
  await page.mouse.move(2, 2);
  const summaryLog = () => consoleLines.map(({ text }) =>
    text.match(/\[tactical-results\] summary lines=(\[.*\])/)).filter(Boolean);
  const deadline = Date.now() + 150_000;
  while (summaryLog().length === 0 && Date.now() < deadline) {
    await page.waitForTimeout(500);
  }
  const summary = summaryLog();
  assert.equal(summary.length, 1, `${faction}: the battle did not end in one Battle Summary`);
  await page.waitForTimeout(500);
  const results = await stableInteractionFrame(page, folder, "battle-last-ship-results");
  const lines = JSON.parse(summary[0][1]);
  const probes = [{ type: "battle-last-ship-report", system: ready.system, lines }];
  // A0 TAC-01-C013: "The Alliance fleet is defeated." / "Imperial forces
  // have broken the blockade of Mon Calamari. The Alliance fleet has been
  // completely destroyed." (FUN_0049b4d0: 0x7003, 0x7016, 0x700b).
  if (alliance) {
    assert.deepEqual(lines, [
      "The Alliance fleet is defeated. ",
      "Imperial forces have broken the blockade of Mon Calamari. The Alliance fleet has been completely destroyed.",
    ]);
  }
  const [dx, dy] = alliance ? [-23, -25] : [40, -22];
  const frame = alliance ? 10335 : 10336;
  const frameSource = decodeIndexedBmp(fs.readFileSync(path.join(
    root, `data/base/ui/strategy-dll/BMP/${frame}.bmp`,
  )));
  const frameCovers = (x, y) => {
    const [r, g, b] = frameSource.pixel(x + 12, y + 13);
    return !(r < 32 && g < 32 && b > 192);
  };
  const textBoxes = [
    { x: 0, y: 4, width: 400, height: 43 },
    // The body's third line ("destroyed.") reaches two rows lower in the
    // port's font (the accepted glyph deviation).
    { x: 0, y: 202, width: 400, height: 96 },
  ];
  const c013 = path.join(root, ".artifacts/interface-parity/a0/TAC-01-C013/original.png");
  if (alliance && native && fs.existsSync(c013)) {
    // The summary's scene, outside the text boxes and the frame, against A0.
    const a0 = PNG.sync.read(fs.readFileSync(c013));
    const port = PNG.sync.read(results);
    const [left, top] = [97 + dx, 87 + dy];
    let checked = 0;
    let differ = 0;
    for (let y = 0; y < 310; y++) {
      for (let x = 0; x < 400; x++) {
        if (textBoxes.some((rect) => x >= rect.x && y >= rect.y
          && x < rect.x + rect.width && y < rect.y + rect.height)) continue;
        if (frameCovers(x, y)) continue;
        const i = ((top + y) * 640 + left + x) * 4;
        checked++;
        if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[i + c])) differ++;
      }
    }
    probes.push({ type: "battle-last-ship-scene-a0", a0: "TAC-01-C013/original.png", checked, differ });
    assert.equal(differ, 0, `${faction}: the summary scene differs from A0 TAC-01-C013 in ${differ} of ${checked} pixels`);
  }
  // The summary is a child window of the Command Center (FUN_0042a640 ->
  // FUN_0046f140), over the live strategy screen, as A0 TAC-01-C013 shows.
  assert.ok(consoleLines.some(({ text }) =>
    text.includes("[tactical-results] summary_window owner=command_center")),
  `${faction}: the Battle Summary did not open over the Command Center`);
  // Not even for the frame that ended the battle: state 0x16 closes the
  // tactical module (FUN_005975a0) before FUN_0042a640 opens the summary.
  assert.ok(!consoleLines.some(({ text }) =>
    text.includes("[tactical-results] summary_window over=tactical_view")),
  `${faction}: the Battle Summary was drawn over the tactical view`);
  if (alliance && native && fs.existsSync(c013)) {
    // Around the window, against A0: the status bar less its three
    // numbers (game state), the left column up to the galaxy display (x 52)
    // less the message rail's lit icons (A0's unread messages), the
    // cockpit's consoles and droids under
    // the window, and the right column under the sector window A0 had
    // open; A0's pointer (x 630.., y 392..411) is left out.
    const a0 = PNG.sync.read(fs.readFileSync(c013));
    const port = PNG.sync.read(results);
    const count = (x0, y0, x1, y1, skip = () => false) => {
      let differ = 0;
      for (let y = y0; y < y1; y++) {
        for (let x = x0; x < x1; x++) {
          if (skip(x, y)) continue;
          const i = (y * 640 + x) * 4;
          if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[i + c])) differ++;
        }
      }
      return differ;
    };
    const inWindow = (x, y) => x >= 62 && x < 532 && y >= 49 && y < 380;
    const strategy = {
      status_bar: count(0, 0, 640, 26, (x, y) => x >= 230 && x < 530 && y >= 8),
      left_column: count(0, 0, 52, 480, (x, y) => x < 32 && y >= 104 && y < 310),
      consoles: count(0, 396, 630, 480),
      right_column: count(540, 0, 630, 396),
    };
    // Every pixel outside the window, to report: the status bar's numbers,
    // the lit rail icons, the galaxy display and A0's sector window differ.
    const outside = count(0, 0, 640, 480, inWindow);
    probes.push({ type: "battle-last-ship-strategy-a0", a0: "TAC-01-C013/original.png", strategy, outside });
    assert.deepEqual(strategy, { status_bar: 0, left_column: 0, consoles: 0, right_column: 0 },
      `${faction}: the strategy screen around the summary differs from A0 TAC-01-C013`);
  }
  // Modal (manual p. 64; FUN_0046f140 sets +0xb8 = 1 as the System
  // Finder's FUN_00460090 does): the System Finder control takes no click.
  const finder = alliance ? [119.5, 416] : [161.5, 446];
  await click(finder[0], finder[1]);
  await page.mouse.move(2, 2);
  await page.waitForTimeout(300);
  assert.ok(!consoleLines.some(({ text }) => text.includes("[interface] command=0x12d")),
    `${faction}: the cockpit took a click while the Battle Summary was open`);
  const held = await stableInteractionFrame(page, folder, "battle-last-ship-modal");
  assert.equal(sha256(held), sha256(results), `${faction}: the strategy screen changed under the summary`);
  probes.push({ type: "battle-last-ship-modal", control: "0x12d", held: true });
  // FUN_00422ce0's WM_KEYDOWN drops F1..F7 while a child with the +0xb8
  // modal bit is up; the summary passes them on (FUN_004710f0). F2 (System
  // Finder), F6 (Message Index) and F7 (Encyclopedia) open nothing.
  for (const key of ["F2", "F6", "F7"]) {
    await page.keyboard.press(key);
    await page.waitForTimeout(200);
  }
  assert.ok(!consoleLines.some(({ text }) => /\[interface\] command=0x(12d|131)\b/.test(text)
    || text.includes("destination=message_index")),
  `${faction}: an F-key opened a window while the Battle Summary was open`);
  const keysHeld = await stableInteractionFrame(page, folder, "battle-last-ship-fkeys");
  assert.equal(sha256(keysHeld), sha256(results), `${faction}: an F-key changed the screen under the summary`);
  probes.push({ type: "battle-last-ship-fkeys", keys: ["F2", "F6", "F7"], held: true });
  // Accelerator table 11 arrives as message 0x483 (FUN_004138a0). Its Agent
  // cases (Alt+G 0xbc5, Alt+H 0xbd7) act only while DAT_006be4e4, the list
  // a +0xb8 window joins on WM_CREATE (FUN_006068f0), is empty; the GID
  // cases (Alt+1..9, 0xbca..0xbd2 -> FUN_00425d00) do not read it
  // (FUN_00422ce0). A frame between key events so the port sees Alt held.
  const nextFrames = () => page.evaluate(() => new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }));
  const altKey = async (code) => {
    await page.keyboard.down("Alt");
    await nextFrames();
    await page.keyboard.down(code);
    await nextFrames();
    await page.keyboard.up(code);
    await page.keyboard.up("Alt");
    await page.waitForTimeout(200);
  };
  await altKey("KeyG");
  await altKey("KeyH");
  assert.ok(!consoleLines.some(({ text }) => /\[agent\] command=.*source=accelerator/.test(text)),
    `${faction}: Alt+G or Alt+H carried out an Agent command while the Battle Summary was open`);
  const altHeld = await stableInteractionFrame(page, folder, "battle-last-ship-alt-agent");
  assert.equal(sha256(altHeld), sha256(results), `${faction}: Alt+G or Alt+H changed the screen under the summary`);
  probes.push({ type: "battle-last-ship-alt-agent", keys: ["Alt+G", "Alt+H"], held: true });
  // The campaign shows Popular Support (Alt+1) already, so Alt+2 picks
  // Uprisings (0x12) and Alt+1 brings Popular Support (0x11) back for the
  // checks below.
  const gidSelected = (id) => consoleLines.some(({ text }) => new RegExp(
    `\\[interface\\] command=0x${id} destination=gid status=selected label=.* source=accelerator`).test(text));
  await altKey("Digit2");
  assert.ok(gidSelected("12"), `${faction}: Alt+2 did not switch the GID display under the summary`);
  const uprisings = await stableInteractionFrame(page, folder, "battle-last-ship-alt-2");
  assert.notEqual(sha256(uprisings), sha256(results), `${faction}: Alt+2 changed no display under the summary`);
  await altKey("Digit1");
  assert.ok(gidSelected("11"), `${faction}: Alt+1 did not switch the GID display under the summary`);
  const popular = await stableInteractionFrame(page, folder, "battle-last-ship-alt-1");
  probes.push({
    type: "battle-last-ship-alt-gid", keys: ["Alt+2", "Alt+1"], answered: true,
    restored_frame: sha256(popular) === sha256(results),
  });
  // The Display page ("Go directly to", the fourth tab), then the system
  // button at window (29, 182), 169 by 96.
  const tabX = (alliance ? 523.5 : 533) + dx;
  const tabY = (alliance ? 344.5 : 360.5) + dy;
  await click(tabX, tabY);
  await page.mouse.move(2, 2);
  await page.waitForTimeout(300);
  const display = await stableInteractionFrame(page, folder, "battle-last-ship-display");
  await click(85 + dx + 29 + 84.5, 74 + dy + 182 + 48);
  await page.mouse.move(offsetX + 639 * scale, offsetY + 240 * scale);
  await page.waitForTimeout(500);
  const routed = await stableInteractionFrame(page, folder, "battle-last-ship-sector");
  assert.notEqual(sha256(display), sha256(routed), `${faction}: the system button did not route`);
  const opened = consoleLines.map(({ text }) =>
    text.match(/\[tactical-results\] sector_window system=(.*) windows=(\d+)/)).filter(Boolean);
  assert.equal(opened.length, 1, `${faction}: the sector window did not open`);
  assert.equal(opened[0][1], ready.system);
  probes.push({ type: "battle-last-ship-sector-window", system: opened[0][1], windows: Number(opened[0][2]) });
  const c014 = path.join(root, ".artifacts/interface-parity/a0/TAC-07-C014/original.png");
  if (alliance && native && fs.existsSync(c014)) {
    const a0 = PNG.sync.read(fs.readFileSync(c014));
    const port = PNG.sync.read(routed);
    const count = (x0, y0, x1, y1) => {
      let differ = 0;
      for (let y = y0; y < y1; y++) {
        for (let x = x0; x < x1; x++) {
          const i = (y * 640 + x) * 4;
          if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[i + c])) differ++;
        }
      }
      return differ;
    };
    // Every pixel, to report; the sectors' planets, bars and names follow
    // each game's state (A0 is day 49 of a played game).
    const whole = count(0, 0, 640, 480);
    // The window's place and chrome: its outline (white at rows 36 and
    // 393), the bands outside it, its right edge and its title buttons.
    // The left edge at x 299 is crossed by the galaxy's "Popular Support"
    // label, and the title's glyphs are the accepted font deviation.
    const chrome = {
      top: count(297, 34, 539, 37),
      bottom: count(297, 393, 539, 395),
      left: count(297, 299, 299, 396) + count(297, 34, 299, 299),
      right: count(536, 34, 539, 396),
      buttons: count(500, 37, 536, 57),
    };
    // The battle's state that persists at Mon Calamari: the surviving
    // Imperial fleet's icon at the planet's top right and the Imperial
    // facility icon at its top left (A0 TAC-07-C014's manifest: "an
    // Imperial fleet icon ... the Alliance fleet icon is gone").
    const monCalamari = {
      imperial_fleet: count(410, 236, 434, 248),
      facility: count(375, 236, 392, 246),
    };
    probes.push({
      type: "battle-last-ship-sector-a0", a0: "TAC-07-C014/original.png", whole, chrome, monCalamari,
    });
    assert.deepEqual(chrome, { top: 0, bottom: 0, left: 0, right: 0, buttons: 0 },
      `${faction}: the sector window's place or chrome differs from A0 TAC-07-C014`);
    assert.deepEqual(monCalamari, { imperial_fleet: 0, facility: 0 },
      `${faction}: Mon Calamari's icons differ from A0 TAC-07-C014`);
  }
  return probes;
}

// Withdraw From Battle (FUN_005f1170, command 0x7a) played out: the
// viewer's one hull leaves the enemy's system, the battle ends when its
// side has no unit left, and the report reads its failed attack and its
// withdrawal (FUN_0049b4d0: 0x7003, 0x7009, 0x700d) over the withdrawal
// scene, as A0 TAC-07-C004 (the Alliance at Coruscant).
async function probeTacticalBattleWithdrawalReport(page, viewport, folder, consoleLines, ready, faction) {
  assert.equal(ready.battle_alert_open, false, `${faction}: the Battle Alert is still open`);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const click = async (x, y) => {
    const at = point(x, y);
    await page.mouse.move(at.x, at.y);
    await page.mouse.down({ button: "left" });
    await page.waitForTimeout(100);
    await page.mouse.up({ button: "left" });
    await page.waitForTimeout(300);
  };
  const alliance = faction === "alliance";
  const native = viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1;
  // Battle Options (A0 TAC-06-C001 positions): open, Withdraw, confirm
  // (1113), close; then the pause control resumes the battle.
  await click(618, 319);
  await click(521, 64);
  await click(561.5, 242.5);
  assert.ok(consoleLines.some(({ text }) =>
    text.includes("[tactical_options] command=withdraw status=withdrawal_started")),
  `${faction}: the withdrawal did not start`);
  await click(593.5, 228.5);
  await click(575, 319);
  await page.mouse.move(2, 2);
  const summaryLog = () => consoleLines.map(({ text }) =>
    text.match(/\[tactical-results\] summary lines=(\[.*\])/)).filter(Boolean);
  const deadline = Date.now() + 150_000;
  while (summaryLog().length === 0 && Date.now() < deadline) {
    await page.waitForTimeout(500);
  }
  const summary = summaryLog();
  assert.equal(summary.length, 1, `${faction}: the battle did not end in one Battle Summary`);
  await page.waitForTimeout(500);
  const results = await stableInteractionFrame(page, folder, "battle-withdrawal-results");
  const lines = JSON.parse(summary[0][1]);
  const [viewer] = alliance ? ["Alliance"] : ["Imperial"];
  // A0 TAC-07-C004: "The Alliance fleet is defeated." / "The Alliance
  // attack on Coruscant has failed. The Alliance fleet has withdrawn to
  // Averam."; the names are the fixture's.
  assert.equal(lines.length, 2, JSON.stringify(lines));
  assert.equal(lines[0], `The ${viewer} fleet is defeated. `);
  const body = lines[1].match(new RegExp(
    `^The ${viewer} attack on (.+) has failed\\. The ${viewer} fleet has withdrawn to (.+)\\.$`));
  assert.ok(body, `${faction}: the report's body ${JSON.stringify(lines[1])}`);
  assert.equal(body[1], ready.system, `${faction}: the report names another system`);
  assert.notEqual(body[2], ready.system, `${faction}: the fleet withdrew to its own battle`);
  const probes = [{ type: "battle-withdrawal-report", lines }];
  const [rx, ry] = alliance ? [-23, -25] : [40, -22];
  const frame = alliance ? 10335 : 10336;
  const scene = alliance ? 10757 : 10758;
  const frameSource = decodeIndexedBmp(fs.readFileSync(path.join(
    root, `data/base/ui/strategy-dll/BMP/${frame}.bmp`,
  )));
  const frameCovers = (x, y) => {
    const [r, g, b] = frameSource.pixel(x + 12, y + 13);
    return !(r < 32 && g < 32 && b > 192);
  };
  const textBoxes = [
    { x: 0, y: 4, width: 400, height: 43 },
    { x: 0, y: 202, width: 400, height: 94 },
  ];
  probes.push({
    type: "battle-withdrawal-scene",
    ...verifyStrategyBitmap(viewport, results, scene, 97 + rx, 87 + ry, textBoxes, false, frameCovers),
  });
  const a0Path = path.join(root, ".artifacts/interface-parity/a0/TAC-07-C004/original.png");
  if (alliance && native && fs.existsSync(a0Path)) {
    const a0 = PNG.sync.read(fs.readFileSync(a0Path));
    const port = PNG.sync.read(results);
    const [left, top] = [97 + rx, 87 + ry];
    let checked = 0;
    let differ = 0;
    for (let y = 0; y < 310; y++) {
      for (let x = 0; x < 400; x++) {
        if (textBoxes.some((rect) => x >= rect.x && y >= rect.y
          && x < rect.x + rect.width && y < rect.y + rect.height)) continue;
        if (frameCovers(x, y)) continue;
        const i = ((top + y) * 640 + left + x) * 4;
        checked++;
        if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[i + c])) differ++;
      }
    }
    probes.push({ type: "battle-withdrawal-scene-a0", a0: "TAC-07-C004/original.png", checked, differ });
    assert.equal(differ, 0, `${faction}: the withdrawal scene differs from A0 TAC-07-C004 in ${differ} of ${checked} pixels`);
  }
  // Escape closes the summary as its Close button does (FUN_004710f0:
  // 0x1b -> slot +0x30, FUN_0041ce80), and the battle's result reaches the
  // galaxy.
  await page.keyboard.press("Escape");
  await page.waitForTimeout(500);
  assert.ok(consoleLines.some(({ text }) => text.includes("[tactical-results] key=escape status=closed")),
    `${faction}: Escape did not close the Battle Summary`);
  assert.ok(consoleLines.some(({ text }) =>
    text.includes("[tactical_results] strategic_persistence applied=true")),
  `${faction}: the result was not applied after Escape`);
  assert.ok(!consoleLines.some(({ text }) => text.includes("[main_menu] returned_from_campaign")),
    `${faction}: Escape left the campaign`);
  const closed = await stableInteractionFrame(page, folder, "battle-withdrawal-escape");
  assert.notEqual(sha256(closed), sha256(results), `${faction}: the summary still shows after Escape`);
  probes.push({ type: "battle-withdrawal-escape", closed: true });
  return probes;
}

async function probeTacticalBattleAlertEntry(
  page, viewport, folder, stable, consoleLines, ready, faction,
) {
  assert.equal(ready.battle_alert_open, true, `${faction}: Battle Alert was bypassed`);
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const alliance = faction === "alliance";
  // A0 TAC-01-C010 and the Imperial Umgul alert: the window opens at
  // (62, 49) / (125, 52); the positions below were measured at (65, 54).
  const [dx, dy] = alliance ? [-3, -5] : [60, -2];
  const resources = alliance ? {
    frame: 10710,
    scene: 10712,
    listScene: 10714,
    rail: 10820,
    tabs: [10729, 10731, 10733, 10735],
    // FUN_0044f860: side 1 takes 0x2ade, 0x29e2 and 0x29e5 (A0
    // TAC-01-C010, battle-clean/10-alert-averam).
    retreat: [10974, 10975],
    simulate: [10722, 10723],
    command: [10725, 10726],
  } : {
    frame: 10711,
    scene: 10713,
    listScene: 10715,
    rail: 10821,
    tabs: [10739, 10741, 10743, 10745],
    retreat: [10971, 10972],
    simulate: [10716, 10717],
    command: [10719, 10720],
  };
  // A0 TAC-01-C010: the keyed frame lies over the scene, so its pixels
  // inside the aperture are checked too; only the scene's text boxes, its
  // bottom row of controls and the rail are skipped.
  const alertFrameSource = decodeIndexedBmp(fs.readFileSync(path.join(
    root, `data/base/ui/strategy-dll/BMP/${resources.frame}.bmp`,
  )));
  const alertFrameCovers = (x, y) => {
    // Only index 252, pure blue, is keyed: 10711's dark blue (0, 0, 223)
    // shows over the scene, as in A0 battle-alert-empire/00-alert-tralus.
    const [r, g, b] = alertFrameSource.pixel(x + 12, y + 13);
    return !(r === 0 && g === 0 && b === 255);
  };
  const probes = [{
    type: "battle-alert-frame",
    ...verifyStrategyBitmap(viewport, stable.bytes, resources.frame, 65 + dx, 54 + dy, [
      { x: 12, y: 13, width: 400, height: 48 },
      { x: 32, y: 213, width: 360, height: 80 },
      { x: 12, y: 293, width: 400, height: 30 },
      { x: 412, y: 0, width: 58, height: 331 },
    ]),
  }, {
    type: "battle-alert-scene",
    // FUN_0044f860 paints the title at (0, 16) and the summary at
    // (36, 216), 328 wide, with its shadow, into the scene.
    ...verifyStrategyBitmap(viewport, stable.bytes, resources.scene, 77 + dx, 67 + dy, [
      { x: 0, y: 0, width: 400, height: 48 },
      { x: 20, y: 200, width: 360, height: 80 },
      { x: 0, y: 280, width: 400, height: 30 },
    ], false, alertFrameCovers),
  }, {
    type: "battle-alert-summary-selected",
    ...verifyStrategyBitmap(
      viewport,
      stable.bytes,
      resources.tabs[0],
      (alliance ? 483 : 491) + dx,
      (alliance ? 75 : 71) + dy,
    ),
  }, {
    type: "battle-alert-retreat-rest",
    ...verifyStrategyBitmap(viewport, stable.bytes, resources.retreat[0], 77 + dx, 350 + dy),
  }, {
    type: "battle-alert-simulate-rest",
    ...verifyStrategyBitmap(viewport, stable.bytes, resources.simulate[0], 211 + dx, 350 + dy),
  }, {
    type: "battle-alert-command-rest",
    ...verifyStrategyBitmap(viewport, stable.bytes, resources.command[0], 345 + dx, 350 + dy),
  }];
  // The three controls at rest against A0, every pixel but the key: the
  // Alliance's TAC-01-C010 (Wine) and the Empire's battle-alert-empire/00
  // (VM), each window at the port's place (FUN_0044f860 picks the art by
  // side).
  const controlsA0 = path.join(root, alliance
    ? ".artifacts/a0-vm/battle-clean/10-alert-averam-day672.png"
    : ".artifacts/a0-vm/battle-alert-empire/00-alert-tralus-day300.png");
  if (fs.existsSync(controlsA0)) {
    const a0 = fs.readFileSync(controlsA0);
    probes.push(...[["retreat", 77], ["simulate", 211], ["command", 345]].map(([name, x]) => ({
      type: `battle-alert-${name}-a0`,
      ...verifyStrategyBitmap(viewport, a0, resources[name][0], x + dx, 350 + dy),
    })));
  }

  // The alert holds the command center (A0 2026-11-05: with the Battle at
  // Umgul alert up, two clicks on the Popular Support rail opened no
  // Message Index and the day readout no speed menu).
  const rail = point(alliance ? 16 : 624, alliance ? 120 : 121);
  for (let press = 0; press < 2; press += 1) {
    await page.mouse.click(rail.x, rail.y);
    await page.waitForTimeout(200);
  }
  await page.waitForTimeout(300);
  assert.ok(!consoleLines.some(({ text }) => text.includes("destination=message_index")),
    `${faction}: the rail opened the Message Index under the Battle Alert`);
  const tabX = (alliance ? 503.5 : 513) + dx;
  const tabY = (alliance ? [95.5, 155.5, 217.5, 279.5] : [91.5, 154.5, 217.5, 280.5]).map((y) => y + dy);
  for (let index = 1; index < 4; index++) {
    const tab = point(tabX, tabY[index]);
    await page.mouse.click(tab.x, tab.y);
    const capture = await stableInteractionFrame(page, folder, `battle-alert-tab-${index}`);
    if (index === 1) {
      // FUN_0044f860 -> FUN_0060a490(.., 7, 0x29fc): five squadrons a side
      // outgrow the 234-high list, so its 10-wide bar (RCDATA 10748) shows
      // at window x 329, its 13-wide arrows at 328 with the outer columns
      // clipped; the thumb, copied into the bar from its column 0 (hyp,
      // list_scroll_bar.rs: A0 messages-day633's thumb one column right of
      // its arrows), at 329 with its last three columns clipped; its top
      // cap starts at the track's top (49).
      const clipped = [{ x: 0, y: 0, width: 1, height: 13 }, { x: 11, y: 0, width: 2, height: 13 }];
      const thumbClipped = [{ x: 10, y: 0, width: 3, height: 13 }];
      probes.push({
        type: "battle-alert-scroll-bar",
        up_arrow: verifyStrategyBitmap(viewport, capture, 10750, 65 + dx + 328, 54 + dy + 40, clipped),
        down_arrow: verifyStrategyBitmap(viewport, capture, 10752, 65 + dx + 328, 54 + dy + 265, clipped),
        thumb_top: verifyStrategyBitmap(viewport, capture, 10754, 65 + dx + 329, 54 + dy + 49, thumbClipped),
      });
      // A click on the down arrow steps the list's 10 pixels (0x44fcd6):
      // the thumb's top leaves 49.
      const arrow = point(65 + dx + 334, 54 + dy + 269);
      await page.mouse.click(arrow.x, arrow.y);
      const stepped = await stableInteractionFrame(page, folder, "battle-alert-list-scrolled");
      let moved = null;
      if (viewport.width === 640 && viewport.height === 480 && viewport.device_scale_factor === 1) {
        for (let y = 50; y < 80 && moved === null; y++) {
          try {
            verifyStrategyBitmap(viewport, stepped, 10754, 65 + dx + 329, 54 + dy + y, thumbClipped);
            moved = y;
          } catch (_) {
            // not at this row
          }
        }
        assert.ok(moved !== null, `${faction}: the alert list's thumb did not move`);
      }
      probes.push({ type: "battle-alert-scroll-step", thumb_top_y: moved });
    }
    probes.push({
      type: "battle-alert-list-scene",
      // FUN_00450660 shows surface 0x33, built by FUN_0044f860 on the
      // paler scene 0x29da / 0x29db, under the title and the list (window
      // (83, 40), 256 x 234, with its bar).
      ...verifyStrategyBitmap(viewport, capture, resources.listScene, 77 + dx, 67 + dy, [
        { x: 0, y: 0, width: 400, height: 48 },
        { x: 71, y: 27, width: 272, height: 236 },
        // Its last 10 of 410 columns sit under the frame (A0
        // alert-imperial-forces).
        { x: 400, y: 0, width: 10, height: 280 },
      ], false, alertFrameCovers),
    });
    // The Empire's System Assets (index 3) against A0
    // battle-alert-empire/01-system-assets (the Battle at Tralus): the
    // fixture's system holds Tralus's five facilities, and the rows' minis
    // and the scene beside them are every pixel A0's. The bar is not
    // compared: A0's list ran on past its fifth row.
    const assetsA0 = path.join(root, ".artifacts/a0-vm/battle-alert-empire/01-system-assets.png");
    if (index === 3 && !alliance && viewport.width === 640 && viewport.height === 480
      && viewport.device_scale_factor === 1 && fs.existsSync(assetsA0)) {
      const a0 = PNG.sync.read(fs.readFileSync(assetsA0));
      const port = PNG.sync.read(capture);
      const differ = ([left, top, right, bottom]) => {
        let count = 0;
        for (let y = top; y < bottom; y++) {
          for (let x = left; x < right; x++) {
            const i = (y * 640 + x) * 4;
            if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[i + c])) count++;
          }
        }
        return count;
      };
      const assets = { minis: differ([220, 120, 272, 320]), scene: differ([140, 120, 215, 330]) };
      assert.deepEqual(assets, { minis: 0, scene: 0 }, `${faction}: System Assets vs A0 ${JSON.stringify(assets)}`);
      probes.push({ type: "battle-alert-system-assets-a0", ...assets });
    }
    probes.push({
      type: "battle-alert-tab",
      tab: ["alliance-forces", "imperial-forces", "system-summary"][index - 1],
      ...verifyStrategyBitmap(
        viewport,
        capture,
        resources.tabs[index],
        (alliance ? 483 : 491) + dx,
        (alliance ? [75, 135, 197, 259][index] : [71, 134, 197, 260][index]) + dy,
      ),
    });
  }

  // The letterboxed run takes the alert's other route: Simulate Results
  // (0x65, FUN_00450540 -> FUN_004362b0 answer 1) resolves the battle and
  // shows its results over the strategy view, as A0 does from the alert
  // (message-rail/battle-averam-result: "The Alliance fleet is defeated.").
  // The Empire's letterboxed run takes Retreat instead (0x67,
  // FUN_00450540 -> FUN_00487830): the same abstract battle with the
  // player's retreat order, then the same results (A0
  // message-rail/retreat-*).
  if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
    const [route, logged, status] = alliance
      ? [point(278 + dx, 363.5 + dy), "command=simulate", "status=routed"]
      : [point(144 + dx, 363.5 + dy), "command=retreat", "status=simulated"];
    const before = await stableInteractionFrame(page, folder, "battle-alert-before-route");
    await page.mouse.move(route.x, route.y);
    await page.mouse.down({ button: "left" });
    await page.waitForTimeout(100);
    await page.mouse.up({ button: "left" });
    await page.waitForTimeout(300);
    await page.mouse.move(2, 2);
    const results = await stableInteractionFrame(page, folder, "battle-alert-routed-results");
    assert.ok(consoleLines.some(({ text }) =>
      text.includes(`[battle_alert] ${logged}`) && text.includes(status)),
    `${faction}: ${logged} did not route the battle`);
    assert.ok(!consoleLines.some(({ text }) => text.includes("[battle_alert] command=take_command")),
      `${faction}: ${logged} entered the tactical battle`);
    assert.notEqual(sha256(results), sha256(before), `${faction}: ${logged} left the alert unchanged`);
    probes.push({ type: "battle-alert-simulate-route", route: logged, screenshot_sha256: sha256(results) });
    return probes;
  }

  const command = point(412 + dx, 363.5 + dy);
  await page.mouse.move(command.x, command.y);
  await page.mouse.down({ button: "left" });
  const held = await dynamicInteractionFrame(page, folder, "battle-alert-command-held");
  probes.push({
    type: "battle-alert-command-held",
    ...verifyStrategyBitmap(viewport, held, resources.command[1], 345 + dx, 350 + dy),
  });
  await page.mouse.up({ button: "left" });
  await page.waitForTimeout(100);
  const tactical = await stableInteractionFrame(page, folder, "battle-alert-take-command");
  probes.push({
    type: "battle-alert-take-command",
    ...verifyTacticalShell(viewport, tactical),
    screenshot_sha256: sha256(tactical),
  });
  assert.ok(consoleLines.some(({ text }) =>
    text.includes("[battle_alert] command=take_command")
      && text.includes("status=paused_tactical")),
  `${faction}: Take Command did not enter paused tactical combat`);
  // FUN_0044f860 -> FUN_00417610(10 / 11): REBEXE's table 0x006a8478 maps
  // them to MDATA.307 (Alliance) and MDATA.313 (Empire).
  const alertScore = consoleLines.find(({ text }) =>
    text.includes("[audio] context=battle_window track=original mdata="));
  assert.ok(alertScore, `${faction}: the Battle Alert started no score`);
  assert.match(alertScore.text, new RegExp(`mdata=${alliance ? 307 : 313} loaded=true`));
  return probes;
}

function probeTacticalAudioRouting(folder, stable, consoleLines, ready) {
  // FUN_00450540 stops the score at the Battle Alert (FUN_00417640); the
  // tactical battle has none (A0 TAC-01-C012).
  assert.equal(ready.tactical_music_mdata_id, null);
  // FUN_005bae60's weapon sounds: fire 0..5 (13000-13017) and the shield
  // and hull hits 0x0c..0x0e and 0x11..0x13 (13030-13038, 13045-13053).
  assert.equal(ready.tactical_weapon_audio_wave_first, 13000);
  assert.equal(ready.tactical_weapon_audio_wave_last, 13053);
  assert.equal(ready.tactical_weapon_audio_variant_count, 36);
  assert.equal(ready.tactical_voice_variant_count, 285);
  assert.equal(ready.battle_alert_open, false);
  const music = consoleLines.find(({ text }) =>
    text.includes("[audio] context=combat track=none source=FUN_00450540"));
  const cueLogs = consoleLines.filter(({ text }) =>
    text.includes("[audio] context=combat event=0x") && text.includes("routed=true"));
  assert.ok(music, "tactical combat did not stop the score");
  assert.match(music.text, /muted=true/);
  const routed = cueLogs.map(({ text }) => {
    const match = text.match(/event=0x([0-9a-f]{2}) wave=(\d+)/);
    assert.ok(match, `malformed tactical cue log: ${text}`);
    assert.match(text, /loaded=true/);
    assert.match(text, /muted=true/);
    return { event: Number.parseInt(match[1], 16), wave: Number(match[2]) };
  });
  const span = (first, last) => Array.from({ length: last - first + 1 }, (_, index) => first + index);
  assert.deepEqual(routed.map(({ wave }) => wave),
    [...span(13000, 13017), ...span(13030, 13038), ...span(13045, 13053)]);
  assert.deepEqual([...new Set(routed.map(({ event }) => event))],
    [0, 1, 2, 3, 4, 5, 0x0c, 0x0d, 0x0e, 0x11, 0x12, 0x13]);
  const voiceLogs = consoleLines.filter(({ text }) =>
    text.includes("[audio] context=combat voice_event=") && text.includes("routed=true"));
  const voices = voiceLogs.map(({ text }) => {
    const match = text.match(/source_event=0x([0-9a-f]+) wave=(\d+) faction=(Alliance|Empire)/);
    assert.ok(match, `malformed tactical voice log: ${text}`);
    assert.match(text, /loaded=true/);
    assert.match(text, /muted=true/);
    return { sourceEvent: Number.parseInt(match[1], 16), wave: Number(match[2]), faction: match[3] };
  });
  const resourceRanges = [
    ["Alliance", 14001, 14122], ["Alliance", 15133, 15163],
    ["Empire", 15001, 15132],
  ];
  const expectedVoices = resourceRanges.flatMap(([faction, first, last]) =>
    Array.from({ length: last - first + 1 }, (_, index) => ({ faction, wave: first + index })));
  assert.deepEqual(voices.map(({ faction, wave }) => ({ faction, wave })), expectedVoices);
  assert.equal(new Set(voices.map(({ faction, wave }) => `${faction}:${wave}`)).size, 285);
  assert.deepEqual(voices.slice(0, 2).map(({ sourceEvent }) => sourceEvent), [0x20, 0x21]);
  assert.equal(voices.find(({ faction, wave }) => faction === "Alliance" && wave === 15133).sourceEvent, 0x11e);
  assert.equal(voices.find(({ faction, wave }) => faction === "Empire" && wave === 15001).sourceEvent, 0x9a);
  fs.writeFileSync(path.join(folder, "tactical-audio-muted.png"), stable.bytes);
  return [{
    type: "tactical-audio-routing",
    music_mdata_id: 307,
    tactical_wave_first: 13000,
    tactical_wave_last: 13053,
    tactical_wave_variants: 36,
    tactical_voice_variants: 285,
    music_log: music.text,
    cue_logs: cueLogs.map(({ text }) => text),
    voice_logs: voiceLogs.map(({ text }) => text),
    browser_muted: true,
    screenshot_sha256: sha256(stable.bytes),
  }];
}

async function probeTacticalBattleOptionsPresentation(
  page, viewport, folder, stable, consoleLines, faction, withdrawalJourney,
) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const panelResource = 1306; // A0 TAC-06-C001: one Battle Options panel
  const controls = [
    // A0 TAC-06-C001 positions.
    { name: "withdraw", x: 500, y: 47, width: 48, height: 34, normal: 1149, pressed: 1150 },
    { name: "simulate", x: 496, y: 121, width: 44, height: 25, normal: 1151, pressed: 1152 },
    { name: "observe", x: 569, y: 121, width: 45, height: 25, normal: 1153, pressed: 1154 },
    { name: "game-options", x: 505, y: 212, width: 52, height: 33, normal: 1155, pressed: 1156 },
    { name: "close", x: 579, y: 212, width: 29, height: 33, normal: 1157, pressed: 1158 },
  ];
  const ignored = (included = controls) => included.map(({ x, y, width, height }) => ({
    x: x - 482, y: y - 28, width, height,
  }));
  const captureHeld = async (control, name) => {
    const location = point(control.x + control.width / 2, control.y + control.height / 2);
    await page.mouse.move(location.x, location.y);
    await page.mouse.down();
    const capture = await stableInteractionFrame(page, folder, name);
    await page.mouse.up();
    return capture;
  };
  const openLocation = point(618, 319);
  await page.mouse.move(openLocation.x, openLocation.y);
  await page.mouse.down();
  const openHeld = await stableInteractionFrame(page, folder, "battle-options-open-held");
  const probes = [{
    type: "battle-options-hud-pressed",
    ...verifyTacticalBitmap(viewport, openHeld, 1039, 608, 309),
  }];
  await page.mouse.up();
  let panel = await stableInteractionFrame(page, folder, "battle-options-open");
  probes.push({
    type: "battle-options-panel",
    faction,
    ...verifyTacticalBitmap(viewport, panel, panelResource, 482, 28, [], ignored()),
  });
  for (const control of controls) {
    probes.push({
      type: "battle-options-control",
      control: control.name,
      ...verifyTacticalBitmap(viewport, panel, control.normal, control.x, control.y),
    });
  }

  if (withdrawalJourney) {
    const withdraw = controls[0];
    const held = await captureHeld(withdraw, "battle-options-withdraw-held");
    probes.push({
      type: "battle-options-withdraw-pressed",
      ...verifyTacticalBitmap(viewport, held, withdraw.pressed, withdraw.x, withdraw.y),
    });
    let confirmation = await stableInteractionFrame(
      page, folder, "battle-options-withdraw-confirmation",
    );
    const confirmationIgnored = [
      { x: 20, y: 28, width: 125, height: 14 },
      { x: 14, y: 92, width: 132, height: 28 },
      { x: 66, y: 202, width: 27, height: 25 },
      { x: 106, y: 202, width: 27, height: 25 },
    ];
    probes.push({
      type: "withdraw-confirmation-panel",
      ...verifyTacticalBitmap(
        viewport, confirmation, 1310, 482, 28, [], confirmationIgnored,
      ),
    });
    for (const control of [
      { name: "confirm", x: 548, y: 230, width: 27, height: 25, normal: 1113, pressed: 1114 },
      { name: "cancel", x: 588, y: 230, width: 27, height: 25, normal: 1115, pressed: 1116 },
    ]) {
      probes.push({
        type: "withdraw-confirmation-control",
        control: control.name,
        ...verifyTacticalBitmap(viewport, confirmation, control.normal, control.x, control.y),
      });
    }
    const cancel = { x: 588, y: 230, width: 27, height: 25, pressed: 1116 };
    const cancelHeld = await captureHeld(cancel, "battle-options-withdraw-cancel-held");
    probes.push({
      type: "withdraw-confirmation-cancel-pressed",
      ...verifyTacticalBitmap(viewport, cancelHeld, cancel.pressed, cancel.x, cancel.y),
    });
    await stableInteractionFrame(page, folder, "battle-options-withdraw-cancelled");

    await page.mouse.click(openLocation.x, openLocation.y);
    await stableInteractionFrame(page, folder, "battle-options-withdraw-reopened");
    await page.mouse.click(point(521, 64).x, point(521, 64).y);
    confirmation = await stableInteractionFrame(
      page, folder, "battle-options-withdraw-confirmation-reopened",
    );
    const confirm = { x: 548, y: 230, width: 27, height: 25, pressed: 1114 };
    const confirmHeld = await captureHeld(confirm, "battle-options-withdraw-confirm-held");
    probes.push({
      type: "withdraw-confirmation-confirm-pressed",
      ...verifyTacticalBitmap(viewport, confirmHeld, confirm.pressed, confirm.x, confirm.y),
    });
    await stableInteractionFrame(page, folder, "battle-options-withdraw-started");
    await page.mouse.click(openLocation.x, openLocation.y);
    panel = await stableInteractionFrame(page, folder, "battle-options-withdraw-disabled");
    // A0 TAC-06-C009: once a side withdraws, FUN_005dab90 hides Withdraw,
    // Get Immediate Result and Observe / Take Command; the panel shows
    // blank there, 0 pixels from 1306.
    probes.push({
      type: "battle-options-withdraw-disabled",
      ...verifyTacticalBitmap(
        viewport, panel, panelResource, 482, 28, [], ignored(controls.slice(3)),
      ),
    });
    const disabledWithdraw = point(521, 64);
    await page.mouse.click(disabledWithdraw.x, disabledWithdraw.y);
    await stableInteractionFrame(page, folder, "battle-options-disabled-click-rejected");
    const withdrawLogs = consoleLines.filter(({ text }) =>
      text.includes("[tactical_options] command=withdraw"));
    assert.equal(withdrawLogs.filter(({ text }) => text.includes("status=confirmation_open")).length,
      2, "withdrawal confirmation did not open exactly twice");
    assert.equal(withdrawLogs.filter(({ text }) =>
      text.includes("status=confirmation_cancelled")).length, 1,
    "withdrawal confirmation cancel did not route exactly once");
    assert.equal(withdrawLogs.filter(({ text }) =>
      text.includes("status=withdrawal_started")).length, 1,
    "disabled withdrawal accepted another command or confirmation did not dispatch");
    assert.match(withdrawLogs.find(({ text }) =>
      text.includes("status=withdrawal_started"))?.text || "", /hyperdrive_warning=true/,
    "withdrawal confirmation did not detect the stranded hyperdrive-disabled ship");
    const expectedVoices = faction === "alliance" ? [
      { name: "WithdrawalStarted", event: "0x8f", wave: 14112, owner: "Alliance" },
      { name: "WithdrawalHyperdriveWarning", event: "0x95", wave: 14118, owner: "Alliance" },
    ] : [
      { name: "WithdrawalStarted", event: "0x10d", wave: 15116, owner: "Empire" },
      { name: "WithdrawalHyperdriveWarning", event: "0x10e", wave: 15117, owner: "Empire" },
    ];
    const voiceLogs = expectedVoices.map((expected) => {
      const voice = consoleLines.find(({ text }) =>
        text.includes(`voice_event=${expected.name}`)
          && text.includes(`source_event=${expected.event}`));
      assert.match(voice?.text || "",
        new RegExp(`wave=${expected.wave} faction=${expected.owner} routed=true loaded=true muted=true`),
        `withdrawal did not route exact muted ${expected.name} voice`);
      return voice.text;
    });
    probes.push({
      type: "battle-options-withdraw-route",
      logs: withdrawLogs.map(({ text }) => text),
      voice_logs: voiceLogs,
    });
    return probes;
  }

  const observe = controls[2];
  let held = await captureHeld(observe, "battle-options-observe-held");
  probes.push({
    type: "battle-options-observe-pressed",
    ...verifyTacticalBitmap(viewport, held, observe.pressed, observe.x, observe.y),
  });
  panel = await stableInteractionFrame(page, folder, "battle-options-observing");
  probes.push({
    type: "battle-options-observe-selected",
    ...verifyTacticalBitmap(viewport, panel, observe.pressed, observe.x, observe.y),
  });
  await captureHeld(observe, "battle-options-take-command-held");
  panel = await stableInteractionFrame(page, folder, "battle-options-take-command");
  probes.push({
    type: "battle-options-take-command",
    ...verifyTacticalBitmap(viewport, panel, observe.normal, observe.x, observe.y),
  });

  const gameOptions = controls[3];
  held = await captureHeld(gameOptions, "battle-options-game-options-held");
  probes.push({
    type: "battle-options-game-options-pressed",
    ...verifyTacticalBitmap(viewport, held, gameOptions.pressed, gameOptions.x, gameOptions.y),
  });
  await stableInteractionFrame(page, folder, "battle-options-game-options-route");
  assert.ok(consoleLines.some(({ text }) =>
    text.includes("[tactical_options] command=game_options status=routed")),
  "Battle Options did not route Game Options");
  const gameOptionsScreen = await stableInteractionFrame(page, folder, "game-options-open");
  probes.push({
    type: "game-options-background",
    ...verifyCommonBitmap(viewport, gameOptionsScreen, 20002, 0, 0, [
      // The full-screen quad's outer two pixels blend with the cleared WebGL
      // framebuffer. Compare the stable source-exact frame around the dynamic
      // labels and controls instead of treating raster coverage as asset drift.
      { x: 0, y: 0, width: 640, height: 2 },
      { x: 0, y: 478, width: 640, height: 2 },
      { x: 0, y: 2, width: 2, height: 476 },
      { x: 638, y: 2, width: 2, height: 476 },
      { x: 22, y: 26, width: 596, height: 440 },
    ]),
  }, {
    // A0 TAC-06-C007: the first display option's lamp 10045 at (357, 311).
    type: "game-options-tactical-toggle-disabled",
    ...verifyCommonBitmap(viewport, gameOptionsScreen, 10045, 357, 311),
  }, {
    // A0 TAC-06-C007: a battle's Game Options shows Restart lit (10035;
    // FUN_00407180 never disables it).
    type: "game-options-restart-lit",
    ...verifyCommonBitmap(viewport, gameOptionsScreen, 10035, 76, 381),
  }, {
    // A0 TAC-06-C007 and the Corsin battle (a0-vm/research-display/34):
    // in a battle every Save is grey (10048) and an occupied slot's Load
    // lit (10049); an empty slot's Load is grey (10051).
    type: "game-options-battle-save-grey",
    ...verifyCommonBitmap(viewport, gameOptionsScreen, 10048, 34, 81),
  }, {
    type: "game-options-battle-load-lit",
    ...verifyCommonBitmap(viewport, gameOptionsScreen, 10049, 287, 81),
  }, {
    type: "game-options-battle-empty-load-grey",
    ...verifyCommonBitmap(viewport, gameOptionsScreen, 10051, 287, 123),
  }, {
    // A0 TAC-06-C007 draws the display options in the system grey
    // (128, 128, 128), the default palette's nearest to 0x2505050.
    type: "game-options-display-options-grey",
    ...(() => {
      if (viewport.width !== 640 || viewport.height !== 480 || viewport.device_scale_factor !== 1) {
        return { status: "non-native-scale" };
      }
      const png = PNG.sync.read(gameOptionsScreen);
      let grey = 0;
      let other = 0;
      for (let y = 312; y < 440; y++) {
        for (let x = 392; x < 610; x++) {
          const offset = (y * png.width + x) * 4;
          const [r, g, b] = png.data.subarray(offset, offset + 3);
          if (r === 128 && g === 128 && b === 128) grey++;
          else if (r > 100 && Math.abs(r - g) < 8 && Math.abs(g - b) < 8 && (r !== 128)) other++;
        }
      }
      assert.ok(grey > 200, `display options text in grey 128: ${grey} (other light ${other})`);
      return { status: "a0-colour", grey_pixels: grey };
    })(),
  });
  assert.ok(consoleLines.some(({ text }) =>
    text.includes("destination=game_options status=opened_original")),
  "Battle Options did not open the original unified Game Options screen");
  const returnToBattle = point(185, 404);
  await page.mouse.click(returnToBattle.x, returnToBattle.y);
  panel = await stableInteractionFrame(page, folder, "game-options-return-to-battle");
  assert.ok(consoleLines.some(({ text }) =>
    text.includes("[game_options] command=return origin=TacticalBattle")),
  "Game Options did not return to the tactical battle");

  const close = controls[4];
  held = await captureHeld(close, "battle-options-close-held");
  probes.push({
    type: "battle-options-close-pressed",
    ...verifyTacticalBitmap(viewport, held, close.pressed, close.x, close.y),
  });
  const closed = await stableInteractionFrame(page, folder, "battle-options-closed");
  assert.notEqual(sha256(panel), sha256(closed), "Battle Options close did not restore the display panel");
  // A0 TAC-06-C008: Close brings back the selection's display panel, its
  // order buttons 1107 and 1105 at (494, 238) and (562, 238).
  probes.push({ type: "battle-options-closed-maneuvers-button",
    ...verifyTacticalBitmap(viewport, closed, 1107, 494, 238) });
  probes.push({ type: "battle-options-closed-missions-button",
    ...verifyTacticalBitmap(viewport, closed, 1105, 562, 238) });

  await page.mouse.click(openLocation.x, openLocation.y);
  await stableInteractionFrame(page, folder, "battle-options-reopened");
  const simulate = controls[1];
  held = await captureHeld(simulate, "battle-options-simulate-held");
  probes.push({
    type: "battle-options-simulate-pressed",
    ...verifyTacticalBitmap(viewport, held, simulate.pressed, simulate.x, simulate.y),
  });
  const results = await stableInteractionFrame(page, folder, "battle-options-simulated-results");
  // A0 TAC-01-C006 / TAC-06-C004 (the Alliance's Battle at Coruscant after
  // Get Immediate Result): the Battle Summary's rail and its five buttons,
  // window (414..464, 11..296), every pixel as A0 draws them, the Display
  // tab's blue corners keyed (10791).
  const simulatedA0 = path.join(root, ".artifacts/interface-parity/a0/TAC-01-C006/original.png");
  if (faction === "alliance" && viewport.width === 640 && viewport.height === 480
    && viewport.device_scale_factor === 1 && fs.existsSync(simulatedA0)) {
    const a0 = PNG.sync.read(fs.readFileSync(simulatedA0));
    const port = PNG.sync.read(results);
    let differing = 0;
    for (let y = 60; y < 345; y++) {
      for (let x = 476; x < 526; x++) {
        const k = (y * 640 + x) * 4;
        if ([0, 1, 2].some((c) => a0.data[k + c] !== port.data[k + c])) differing++;
      }
    }
    probes.push({ type: "battle-options-simulated-rail-a0", differing });
    assert.equal(differing, 0, `the simulated result's rail differs from A0 in ${differing} pixels`);
  }
  probes.push({
    type: "battle-options-simulate-route",
    // A0 TAC-07-C001 / TAC-07-C002: (62, 49) for the Alliance, (125, 52) for the Empire.
    ...verifyStrategyBitmap(viewport, results, faction === "alliance" ? 10335 : 10336,
      faction === "alliance" ? 62 : 125, faction === "alliance" ? 49 : 52, [
      { x: 12, y: 13, width: 400, height: 310 },
      { x: 412, y: 0, width: 58, height: 330 },
    ]),
  });
  const optionLogs = consoleLines
    .filter(({ text }) => text.includes("[tactical_options]"))
    .map(({ text }) => text);
  for (const expected of ["command=observe", "command=take_command", "command=game_options",
    "command=close", "command=simulate_remainder"]) {
    assert.ok(optionLogs.some((line) => line.includes(expected)),
      `missing Battle Options transition ${expected}`);
  }
  probes.push({ type: "battle-options-routes", logs: optionLogs });
  return probes;
}

function probeTacticalSubsystemFieldCommandPresentation(viewport, folder, stable, ready) {
  const probes = probeTacticalSelectedDamagePresentation(viewport, folder, stable, ready);
  const selected = ready.selected_ship;
  const expectedPercentages = [0, 24, 25, 50, 75];
  const expectedResources = [1201, 1207, 1213, 1219, 1225];
  assert.deepEqual(selected.subsystem_percentages, expectedPercentages);
  assert.deepEqual(selected.subsystem_resources, expectedResources);

  const subsystemProofs = expectedResources.map((resourceId, index) =>
    verifyTacticalBitmap(viewport, stable.bytes, resourceId, 492 + index * 27, 131)); // A0 TAC-03-C001
  const tractor = ready.fields.find((field) => field.target === 1);
  const gravity = ready.fields.find((field) => field.target === 4);
  assert.deepEqual(tractor, {
    target: 1,
    resource_id: 3624,
    frame: 4,
    tractor_sources: 2,
    gravity_sources: 0,
    tractor_source_ids: [3, 5],
    gravity_source_ids: [],
  });
  assert.deepEqual(gravity, {
    target: 4,
    resource_id: 3524,
    frame: 4,
    tractor_sources: 1,
    gravity_sources: 1,
    tractor_source_ids: [0],
    gravity_source_ids: [2],
  });
  fs.writeFileSync(path.join(folder, "subsystem-field-command.png"), stable.bytes);
  probes.push({
    type: "source-traced-subsystem-field-command-state",
    executable_functions: [
      "FUN_005e45f0", "FUN_005e7540", "FUN_005e77c0",
      "FUN_005b23e0", "FUN_005b2440", "FUN_005b24d0", "FUN_005b2480",
      "FUN_005b2520", "FUN_005b2550", "FUN_005b2570", "FUN_005b25d0",
    ],
    subsystem_order: ["shields", "weapons", "tractor", "engines", "hyperdrive"],
    percentages: expectedPercentages,
    resources: expectedResources,
    subsystem_proofs: subsystemProofs,
    tractor_source_limit: 1,
    gravity_target_limit_per_source: 4,
    fields: [tractor, gravity],
  });
  return probes;
}

async function probeTacticalGroupPresentation(
  page, viewport, folder, stable, consoleLines, ready, faction,
) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const capture = async (name) => stableInteractionFrame(page, folder, name);
  const labelMask = [{ x: 4, y: 2, width: 17, height: 14 }];
  const playerFighter = ready.participants.find((participant) =>
    participant.kind === "fighter-group" && participant.faction === faction
      && participant.fighter_group === 3);
  assert.ok(playerFighter, "group fixture omitted the player's gold fighter group");
  const portraitResource = 2030 + playerFighter.tactical_ordinal - 29;
  const taskButtons = Array.from({ length: 8 }, (_, index) => ({
    id: index === 0 ? 1006 : 1005,
    x: 60 + index * 29, // A0 TAC-02-C002
    y: 2,
  }));
  const fighterButtons = [1012, 1013, 1014, 1015].map((id, index) => ({
    id, x: 347 + index * 29, y: 2,
  }));
  const probes = [];
  probes.push({
    type: "task-force-header",
    ...verifyTacticalBitmap(viewport, stable.bytes, faction === "alliance" ? 1001 : 1004,
      9, 1, taskButtons),
  });
  probes.push({
    type: "fighter-group-header",
    ...verifyTacticalBitmap(viewport, stable.bytes, faction === "alliance" ? 1008 : 1010,
      296, 1, fighterButtons),
  });
  for (const button of taskButtons) {
    probes.push({
      type: "task-force-initial",
      slot: (button.x - 60) / 29 + 1,
      ...verifyTacticalBitmap(
        viewport, stable.bytes, button.id, button.x, button.y, [], labelMask,
      ),
    });
  }
  probes.push({ type: "task-force-1-number-selected",
    ...verifyTaskForceOneNumber(viewport, stable.bytes, true) });
  for (const [index, button] of fighterButtons.entries()) {
    probes.push({
      type: "fighter-group-initial",
      slot: index + 1,
      ...verifySquadronButton(viewport, stable.bytes, button.id, button.x, button.y),
    });
  }

  const matte = point(347.1, 2.1); // top-left matte pixel of the first squadron button
  await page.mouse.click(matte.x, matte.y);
  await page.waitForTimeout(80);
  const matteFrame = await capture("group-transparent-matte");
  assert.equal(sha256(matteFrame), sha256(stable.bytes),
    "transparent fighter-group matte changed the paused battle");

  const taskEight = point(275, 10);
  await page.mouse.move(taskEight.x, taskEight.y);
  await page.mouse.down({ button: "left" });
  await page.waitForTimeout(80);
  const taskEightHeld = await capture("task-force-8-held");
  probes.push({ type: "task-force-8-held", ...verifyTacticalBitmap(
    viewport, taskEightHeld, 1006, 263, 2, [], labelMask,
  ) });
  await page.mouse.up({ button: "left" });
  await page.waitForTimeout(80);
  const taskEightSelected = await capture("task-force-8-selected");
  probes.push({ type: "task-force-8-selected", ...verifyTacticalBitmap(
    viewport, taskEightSelected, 1006, 263, 2, [], labelMask,
  ) });
  probes.push({ type: "task-force-1-released", ...verifyTacticalBitmap(
    viewport, taskEightSelected, 1005, 60, 2, [], labelMask,
  ) });
  probes.push({ type: "task-force-1-number-at-rest",
    ...verifyTaskForceOneNumber(viewport, taskEightSelected, false) });
  // A0 TAC-02-C010: Task Force 1 held looks selected (1006, a white '1'),
  // the same pixels as A0 TAC-02-C011 but the pointer's.
  const taskOne = point(72, 10);
  await page.mouse.move(taskOne.x, taskOne.y);
  await page.mouse.down({ button: "left" });
  await page.waitForTimeout(80);
  const taskOneHeld = await capture("task-force-1-held");
  probes.push({ type: "task-force-1-held", ...verifyTacticalBitmap(
    viewport, taskOneHeld, 1006, 60, 2, [], labelMask,
  ) });
  probes.push({ type: "task-force-1-number-held",
    ...verifyTaskForceOneNumber(viewport, taskOneHeld, true) });
  await page.mouse.up({ button: "left" });
  await page.waitForTimeout(300);
  await page.mouse.click(taskEight.x, taskEight.y);
  await page.waitForTimeout(300);
  probes.push({ type: "task-force-8-reselected", ...verifyTacticalBitmap(
    viewport, await capture("task-force-8-reselected"), 1006, 263, 2, [], labelMask,
  ) });

  const gold = point(446, 10);
  await page.mouse.move(gold.x, gold.y);
  await page.mouse.down({ button: "left" });
  await page.waitForTimeout(80);
  const goldHeld = await capture("gold-group-held");
  probes.push({ type: "gold-group-held", ...verifySquadronButton(
    viewport, goldHeld, 1019, 434, 2,
  ) });
  await page.mouse.up({ button: "left" });
  await page.waitForTimeout(80);
  const goldSelected = await capture("gold-group-selected");
  probes.push({ type: "gold-group-selected", ...verifySquadronButton(
    viewport, goldSelected, 1019, 434, 2,
  ) });
  probes.push({ type: "selected-fighter-panel", ...verifyTacticalBitmap(
    viewport, goldSelected, 1307, 482, 28,
    [{ id: portraitResource, x: 505, y: 87 }], // A0 TAC-02-C002
    [
      { x: 8, y: 7, width: 145, height: 40 },
      // The count line over the portrait's foot (label +0xb0 at panel
      // (0x17, 0x92), ghidra/notes/tactical-panel-text.md).
      { x: 23, y: 146, width: 102, height: 12 },
      { x: 12, y: 210, width: 126, height: 22 },
    ],
  ) });
  probes.push({ type: "selected-fighter-portrait", ...verifyTacticalBitmap(
    viewport, goldSelected, portraitResource, 505, 87, [],
    [{ x: 0, y: 87, width: 102, height: 12 }],
  ) });

  const participantLine = consoleLines.find(({ text }) =>
    text.includes("[tactical_3d] participant_scene"));
  const participantPositions = parseTacticalScenePositions(
    participantLine, "mixed-task-force participant scene",
  );
  const playerCapitals = ready.participants
    .map((participant, index) => ({ participant, objectId: index + 1 }))
    .filter(({ participant }) => participant.kind === "capital-ship"
      && participant.faction === faction && participant.active);
  const addressablePlayerCapitals = playerCapitals
    .filter(({ objectId }) => {
      const position = participantPositions.get(objectId);
      return position
        && position.x >= offsetX + 40 * scale && position.x < offsetX + 430 * scale
        && position.y >= offsetY + 40 * scale && position.y < offsetY + 420 * scale;
    })
    .sort((left, right) => {
      const leftPosition = participantPositions.get(left.objectId);
      const rightPosition = participantPositions.get(right.objectId);
      const centerX = offsetX + 238 * scale;
      const centerY = offsetY + 247 * scale;
      const distance = (position) => (position.x - centerX) ** 2 + (position.y - centerY) ** 2;
      return distance(leftPosition) - distance(rightPosition);
    });
  const firstCapital = addressablePlayerCapitals[0];
  const secondCapital = addressablePlayerCapitals.find(({ participant }) =>
    participant.task_force !== firstCapital?.participant.task_force);
  const hostileCapital = ready.participants
    .map((participant, index) => ({ participant, objectId: index + 1 }))
    .find(({ participant }) => participant.kind === "capital-ship"
      && participant.faction !== faction && participant.active);
  assert.ok(firstCapital && secondCapital && hostileCapital,
    "group fixture omitted mixed player task forces or a hostile focus target");
  const firstPosition = participantPositions.get(firstCapital.objectId);
  const secondPosition = participantPositions.get(secondCapital.objectId);
  const hostilePosition = participantPositions.get(hostileCapital.objectId);
  assert.ok(firstPosition && secondPosition && hostilePosition,
    "mixed-task-force journey omitted a required projected position");

  await page.mouse.click(firstPosition.x, firstPosition.y);
  await page.waitForTimeout(80);
  await page.keyboard.down("ControlLeft");
  await page.waitForTimeout(80);
  await page.mouse.click(secondPosition.x, secondPosition.y);
  await page.waitForTimeout(80);
  await page.keyboard.up("ControlLeft");
  await page.waitForTimeout(80);
  const selectionLogs = consoleLines
    .filter(({ text }) => text.includes("[tactical_3d] selection object_id="))
    .map(({ text }) => text);
  assert.ok(selectionLogs.some((line) => line.includes(`object_id=${firstCapital.objectId} `))
      && selectionLogs.some((line) => line.includes(`object_id=${secondCapital.objectId} `)),
  "mixed-task-force journey did not select both projected capitals");
  await capture("mixed-task-force-selection");
  const assignedFocusCount = consoleLines.filter(({ text }) =>
    text.includes("[tactical_3d] focus source_object_ids=")).length;
  const assignedTargetCount = consoleLines.filter(({ text }) =>
    text.includes("[tactical_navigation] event=target_assign status=assigned")).length;
  await page.mouse.click(hostilePosition.x, hostilePosition.y, { button: "right" });
  await page.waitForTimeout(120);
  await page.mouse.move(secondPosition.x, secondPosition.y);
  await page.waitForTimeout(80);
  const rejectedTarget = await capture("mixed-task-force-target-rejected");
  assert.equal(consoleLines.filter(({ text }) =>
    text.includes("[tactical_3d] focus source_object_ids=")).length, assignedFocusCount,
  "rejected mixed-task-force target entered the focus-assignment path");
  assert.equal(consoleLines.filter(({ text }) =>
    text.includes("[tactical_navigation] event=target_assign status=assigned")).length,
  assignedTargetCount, "rejected mixed-task-force target emitted an assigned order");
  const rejectionLog = consoleLines.find(({ text }) =>
    text.includes("[tactical_navigation] event=target_assign")
      && text.includes("status=rejected_mixed_task_forces"));
  assert.match(rejectionLog?.text || "", /members=2/,
    "mixed-task-force target did not report the exact rejected selection");
  const expectedVoice = faction === "alliance"
    ? { event: "0x84", wave: 14101, owner: "Alliance" }
    : { event: "0x102", wave: 15105, owner: "Empire" };
  const rejectionVoice = consoleLines.find(({ text }) =>
    text.includes("voice_event=TargetTaskForceRejected")
      && text.includes(`source_event=${expectedVoice.event}`));
  assert.match(rejectionVoice?.text || "",
    new RegExp(`wave=${expectedVoice.wave} faction=${expectedVoice.owner} routed=true loaded=true muted=true`),
    "mixed-task-force rejection did not route the exact muted faction voice");
  probes.push({
    type: "source-rejected-mixed-task-force-target",
    executable_function: "FUN_005a24d0",
    player_task_forces: [firstCapital.participant.task_force, secondCapital.participant.task_force],
    hostile_object_id: hostileCapital.objectId,
    rejection_log: rejectionLog.text,
    voice_log: rejectionVoice.text,
    source_event: expectedVoice.event,
    resource_id: expectedVoice.wave,
    assignment_state_unchanged: true,
    screenshot_sha256: sha256(rejectedTarget),
  });

  // 0x005a0d10: a left click on an enemy capital ship selects it, into
  // the view-only enemy state 0x66bba8 (FUN_005a1110).
  await page.mouse.click(hostilePosition.x, hostilePosition.y);
  await page.waitForTimeout(120);
  const enemySelected = await capture("enemy-capital-selected");
  const enemySelectionLog = consoleLines.find(({ text }) =>
    text.includes(`[tactical_3d] selection object_id=${hostileCapital.objectId} `));
  assert.ok(enemySelectionLog, "a left click did not select the enemy capital ship");
  assert.notEqual(sha256(enemySelected), sha256(rejectedTarget),
    "the enemy selection changed nothing on screen");
  probes.push({
    type: "source-enemy-capital-selection",
    executable_functions: ["0x005a0d10", "FUN_005a1110"],
    hostile_object_id: hostileCapital.objectId,
    selection_log: enemySelectionLog.text,
    screenshot_sha256: sha256(enemySelected),
  });

  // FUN_0059b730: keys 1-8 press task-force buttons 0xad02-0xad09;
  // F1-F12 only move the camera (FUN_005c2990 mode 0xb).
  await page.keyboard.press("Digit1");
  await page.waitForTimeout(80);
  const taskOneHotkey = await capture("task-force-1-hotkey");
  probes.push({ type: "task-force-1-hotkey", ...verifyTacticalBitmap(
    viewport, taskOneHotkey, 1006, 60, 2, [], labelMask,
  ) });
  // Space (0x20, slot +0x1c, FUN_005aa970): the selection targets its
  // nearest enemy.
  await page.keyboard.press("Space");
  await page.waitForTimeout(80);
  const nearestLog = consoleLines.find(({ text }) =>
    text.includes("[tactical_navigation] event=nearest_target"));
  assert.match(nearestLog?.text || "", /members=[1-9]/,
    "Space did not target the selection's nearest enemy");
  probes.push({ type: "source-space-nearest-target", executable_function: "FUN_005aa970",
    log: nearestLog.text });
  await page.keyboard.press("F12");
  await page.waitForTimeout(80);
  const followHotkey = await capture("camera-follow-f12-hotkey");
  probes.push({ type: "camera-follow-f12-keeps-selection", ...verifyTacticalBitmap(
    viewport, followHotkey, 1006, 60, 2, [], labelMask,
  ) });

  assert.ok(consoleLines.some(({ text }) =>
    text.includes("[tactical_groups] task_force_selected slot=8 members=1")),
  "task-force selection did not emit its deterministic state transition");
  assert.ok(consoleLines.some(({ text }) =>
    text.includes("[tactical_groups] fighter_group_selected slot=4 members=1")),
  "fighter-group selection did not emit its deterministic state transition");
  probes.push({
    type: "source-task-force-fighter-group-presentation",
    executable_functions: ["FUN_005c7150", "FUN_0059f680", "FUN_005ab650"],
    task_force_resources: { normal: 1005, pressed: 1006, unassigned: 1007 },
    fighter_group_resources: { normal: [1012, 1013, 1014, 1015], pressed: [1016, 1017, 1018, 1019] },
    fighter_texture_resources: ready.participants
      .filter(({ kind }) => kind === "fighter-group")
      .map(({ faction: owner, fighter_group, active_close_resource,
        active_far_resource, active_indicator_resource }) => ({
        faction: owner,
        fighter_group,
        close: active_close_resource,
        far: active_far_resource,
        indicator: active_indicator_resource,
      })),
    keyboard: { task_forces: "1-8", camera_follow: "F1-F12" },
  });
  return probes;
}

async function probeTacticalSelectedNavigationPresentation(
  page, viewport, folder, stable, consoleLines, ready, faction,
) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const capture = async (name) => stableInteractionFrame(page, folder, name);
  const playerCapitals = ready.participants.filter(({ kind, faction: owner, active, task_force }) =>
    kind === "capital-ship" && owner === faction && active && task_force === 0);
  assert.equal(playerCapitals.length, 3,
    "selected-navigation fixture must expose three live capitals in task force 1");

  const probes = [
    {
      type: "previous-capital-rest",
      ...verifyTacticalBitmap(viewport, stable.bytes, 1101, 489, 38),
    },
    {
      type: "next-capital-rest",
      ...verifyTacticalBitmap(viewport, stable.bytes, 1103, 614, 38),
    },
  ];
  const press = async (name, location, pressedResource, pressedX) => {
    await page.mouse.move(location.x, location.y);
    await page.mouse.down({ button: "left" });
    await page.waitForTimeout(80);
    const held = await capture(`${name}-held`);
    probes.push({
      type: `${name}-held`,
      ...verifyTacticalBitmap(viewport, held, pressedResource, pressedX, 38),
    });
    await page.mouse.up({ button: "left" });
    await page.waitForTimeout(80);
    const released = await capture(`${name}-released`);
    probes.push({
      type: `${name}-released`,
      framebuffer_sha256: sha256(released),
    });
  };

  await press("next-capital", point(618, 60), 1104, 614);
  await press("previous-capital", point(493, 60), 1102, 489);
  await press("previous-capital-wrap", point(493, 60), 1102, 489);
  const transitions = consoleLines
    .filter(({ text }) => text.includes("[tactical_selection]"))
    .map(({ text }) => text);
  assert.equal(transitions.length, 3,
    "selected-capital navigation did not emit all three transitions");
  assert.match(transitions[0], /direction=next task_force=1 from=(\d+) to=(\d+)/);
  assert.match(transitions[1], /direction=previous task_force=1 from=(\d+) to=(\d+)/);
  assert.match(transitions[2], /direction=previous task_force=1 from=(\d+) to=(\d+)/);
  const parse = (line) => line.match(/from=(\d+) to=(\d+)/).slice(1).map(Number);
  const [nextFrom, nextTo] = parse(transitions[0]);
  const [returnFrom, returnTo] = parse(transitions[1]);
  const [wrapFrom, wrapTo] = parse(transitions[2]);
  assert.deepEqual([returnFrom, returnTo], [nextTo, nextFrom],
    "previous control did not reverse the next-capital transition");
  assert.equal(wrapFrom, nextFrom);
  assert.notEqual(wrapTo, wrapFrom, "previous control did not wrap to another task-force member");
  probes.push({
    type: "source-task-force-selected-capital-navigation",
    constructor_functions: ["FUN_005e4110", "FUN_005e7030"],
    task_force: 1,
    members: playerCapitals.map(({ fleet_roster_index, class_dat_id }) => ({
      fleet_roster_index,
      class_dat_id,
    })),
    controls: {
      previous: { rect: [489, 38, 11, 53], resources: [1101, 1102] },
      next: { rect: [614, 38, 11, 53], resources: [1103, 1104] },
    },
    transitions,
    wraps_within_task_force: true,
  });
  return probes;
}

function parseTacticalNavigationScene(consoleLines) {
  const line = [...consoleLines].reverse().find(({ text }) =>
    text.includes("[tactical_navigation] event=scene")
      && /screen_positions=\S+/.test(text));
  const match = line?.text.match(/screen_positions=([^ ]+)/);
  assert.ok(match, "navigation scene omitted projected point positions");
  return match[1].split(";").map((entry) => {
    const fields = entry.match(/^(\d+):(\d+):(-?[\d.]+),(-?[\d.]+)$/);
    assert.ok(fields, `invalid navigation point projection ${entry}`);
    return {
      set: Number(fields[1]),
      point: Number(fields[2]),
      x: Number(fields[3]),
      y: Number(fields[4]),
    };
  });
}

async function probeTacticalNavigationCameraPresentation(
  page, viewport, folder, stable, consoleLines, ready,
) {
  const scale = Math.min(viewport.width / 640, viewport.height / 480);
  const offsetX = (viewport.width - 640 * scale) / 2;
  const offsetY = (viewport.height - 480 * scale) / 2;
  const point = (x, y) => ({ x: offsetX + x * scale, y: offsetY + y * scale });
  const capture = async (name) => stableInteractionFrame(page, folder, name);
  const press = async (name, location, resource, resourceX, resourceY) => {
    await page.mouse.move(location.x, location.y);
    await page.mouse.down({ button: "left" });
    await page.waitForTimeout(80);
    const held = await capture(`${name}-held`);
    const bitmap = verifyTacticalBitmap(viewport, held, resource, resourceX, resourceY);
    await page.mouse.up({ button: "left" });
    await page.waitForTimeout(80);
    return { bitmap, released: await capture(`${name}-released`) };
  };
  const probes = [];

  const navControls = [
    { set: 1, normal: 1026, selected: 1030, x: 485 },
    { set: 2, normal: 1027, selected: 1031, x: 524 },
    { set: 3, normal: 1028, selected: 1032, x: 563 },
    { set: 4, normal: 1029, selected: 1033, x: 602 },
  ];
  for (const control of navControls) {
    probes.push({
      type: `navigation-set-${control.set}-rest`,
      ...verifyTacticalBitmap(viewport, stable.bytes, control.normal, control.x, 272),
    });
    await page.mouse.click(point(control.x + 13, 285).x, point(control.x + 13, 285).y);
    await page.waitForTimeout(80);
    const selected = await capture(`navigation-set-${control.set}-selected`);
    probes.push({
      type: `navigation-set-${control.set}-selected`,
      ...verifyTacticalBitmap(viewport, selected, control.selected, control.x, 272),
    });
  }

  // The original's grids (FUN_0059f800) put points under ships too; a click
  // there picks the ship, so the journey takes points clear of every
  // participant.
  const sceneLine = consoleLines.find(({ text }) =>
    text.includes("[tactical_3d] participant_scene"));
  const occupied = sceneLine
    ? [...parseTacticalScenePositions(sceneLine, "navigation participant scene").values()]
    : [];
  const navigationPoints = parseTacticalNavigationScene(consoleLines)
    .filter(({ x, y }) => x >= offsetX + 16 * scale && x < offsetX + 460 * scale
      && y >= offsetY + 28 * scale && y < offsetY + 467 * scale)
    .filter(({ x, y }) => occupied.every((position) =>
      Math.hypot(position.x - x, position.y - y) > 24 * scale));
  assert.ok(navigationPoints.length >= 2,
    "navigation journey did not project two addressable points inside the aperture");
  await page.mouse.click(navigationPoints[0].x, navigationPoints[0].y, { button: "right" });
  await page.waitForTimeout(80);
  await page.keyboard.down("ControlLeft");
  await page.waitForTimeout(80);
  await page.mouse.click(navigationPoints[1].x, navigationPoints[1].y, { button: "right" });
  await page.waitForTimeout(80);
  await page.keyboard.up("ControlLeft");
  await page.waitForTimeout(80);
  const routeFrame = await capture("navigation-route-two-points");
  const routeLogs = consoleLines
    .filter(({ text }) => text.includes("[tactical_navigation] event=route_assign"))
    .map(({ text }) => text);
  assert.ok(routeLogs.some((line) => /append=false .*route_len=1/.test(line)),
    "navigation journey did not replace the route with its first point");
  assert.ok(routeLogs.some((line) => /append=true .*route_len=2/.test(line)),
    "navigation journey did not append its second point");
  assert.notEqual(sha256(routeFrame), sha256(stable.bytes),
    "ordered navigation route produced no framebuffer evidence");
  // FUN_005a0840 / FUN_0059f560: the route's first point turns the yellow
  // mesh, its second the magenta one (A0 TAC-04-C002, TAC-04-C003), and
  // the panel reads "Navigating" (FUN_005e4b40, A0 TAC-04-C002).
  const routePng = PNG.sync.read(routeFrame);
  const markerPixels = ({ x, y }, test) => {
    let count = 0;
    const reach = Math.ceil(8 * scale);
    for (let dy = -reach; dy <= reach; dy++) for (let dx = -reach; dx <= reach; dx++) {
      const px = Math.round(x) + dx, py = Math.round(y) + dy;
      const i = (py * routePng.width + px) * 4;
      if (test(routePng.data[i], routePng.data[i + 1], routePng.data[i + 2])) count++;
    }
    return count;
  };
  const yellow = (r, g, b) => r >= 120 && g >= 120 && b <= 40 && Math.abs(r - g) <= 30;
  const magenta = (r, g, b) => r >= 100 && b >= 100 && g <= 30 && Math.abs(r - b) <= 30;
  const firstYellow = markerPixels(navigationPoints[0], yellow);
  const secondMagenta = markerPixels(navigationPoints[1], magenta);
  assert.ok(firstYellow >= 3, `the route's first point shows ${firstYellow} yellow pixels`);
  assert.ok(secondMagenta >= 3, `the route's second point shows ${secondMagenta} magenta pixels`);
  if (Math.hypot(navigationPoints[0].x - navigationPoints[1].x,
    navigationPoints[0].y - navigationPoints[1].y) > 16 * scale) {
    assert.equal(markerPixels(navigationPoints[0], magenta), 0, "the first point is not magenta");
  }
  const routeOrders = consoleLines.map(({ text }) => text.match(/\[tactical_panel\] orders_line=(.*)$/))
    .filter(Boolean).map((match) => match[1]);
  assert.equal(routeOrders.at(-1), "Navigating", JSON.stringify(routeOrders));
  probes.push({ type: "navigation-route-markers", first_yellow_pixels: firstYellow,
    second_magenta_pixels: secondMagenta, orders_line: routeOrders.at(-1) });

  const participantLine = consoleLines.find(({ text }) =>
    text.includes("[tactical_3d] participant_scene"));
  const participantPositions = parseTacticalScenePositions(
    participantLine, "navigation participant scene",
  );
  const hostileObjectIds = ready.participants
    .map((participant, index) => ({ participant, objectId: index + 1 }))
    .filter(({ participant }) => participant.kind === "capital-ship"
      && participant.faction !== ready.faction)
    .map(({ objectId }) => objectId);
  const enemyObjectIds = hostileObjectIds.length >= 2
    ? [hostileObjectIds[0], hostileObjectIds[hostileObjectIds.length - 1]]
    : hostileObjectIds;
  assert.equal(enemyObjectIds.length, 2,
    "navigation fixture omitted two hostile capital targets");
  const enemyTargets = enemyObjectIds.map((objectId) => participantPositions.get(objectId));
  assert.ok(enemyTargets.every(Boolean), "hostile target projections are incomplete");
  await page.mouse.click(enemyTargets[0].x, enemyTargets[0].y, { button: "right" });
  await page.waitForTimeout(80);
  await page.keyboard.down("ControlLeft");
  await page.waitForTimeout(80);
  await page.mouse.click(enemyTargets[1].x, enemyTargets[1].y, { button: "right" });
  await page.waitForTimeout(80);
  await page.keyboard.up("ControlLeft");
  await page.waitForTimeout(80);
  const multipleTargets = await capture("multiple-targets");
  const targetLogs = consoleLines
    .filter(({ text }) => text.includes("[tactical_navigation] event=target_assign"))
    .map(({ text }) => text);
  assert.ok(targetLogs.some((line) => /append=true .*target_count=2/.test(line)),
    "navigation journey did not retain two ordered hostile targets");
  assert.notEqual(sha256(multipleTargets), sha256(routeFrame),
    "multiple targets produced no additional framebuffer evidence");

  await page.mouse.click(point(503, 355).x, point(503, 355).y);
  await page.waitForTimeout(80);
  const memorizedView = await capture("camera-view-to-memorize");
  const memorized = await press("camera-memorize", point(615, 456), 1041, 594, 444);
  await page.mouse.click(point(619, 355).x, point(619, 355).y);
  await page.mouse.click(point(506, 402).x, point(506, 402).y);
  await page.waitForTimeout(80);
  const changedView = await capture("camera-view-changed");
  assert.notEqual(tacticalApertureHash(viewport, changedView),
    tacticalApertureHash(viewport, memorizedView),
    "camera changes did not alter the tactical aperture");
  const recalled = await press("camera-recall", point(507, 456), 1043, 486, 444);
  assert.equal(tacticalApertureHash(viewport, recalled.released),
    tacticalApertureHash(viewport, memorized.released),
    "camera recall did not restore the memorized retained-mode view");

  await page.keyboard.press("F1");
  await page.waitForTimeout(100);
  const chase = await capture("camera-chase-task-force-1");
  await page.mouse.click(point(506, 402).x, point(506, 402).y);
  await page.waitForTimeout(80);
  const chaseExited = await capture("camera-chase-exited");
  assert.notEqual(tacticalApertureHash(viewport, chase),
    tacticalApertureHash(viewport, recalled.released),
    "task-force chase did not change the retained-mode target");
  assert.notEqual(tacticalApertureHash(viewport, chaseExited),
    tacticalApertureHash(viewport, chase),
    "manual camera control did not exit the chase view");

  const cameraLogs = consoleLines
    .filter(({ text }) => text.includes("[tactical_camera]"))
    .map(({ text }) => text);
  assert.ok(cameraLogs.some((line) => /command=memorize stored=true/.test(line)));
  assert.ok(cameraLogs.some((line) => /command=recall source=memorized/.test(line)));
  assert.ok(cameraLogs.some((line) => /command=follow kind=task_force group=1/.test(line)));
  assert.ok(cameraLogs.some((line) => /command=follow_exit reason=manual/.test(line)));
  probes.push({
    type: "tactical-navigation-camera-journey",
    navigation_resources: navControls.flatMap(({ normal, selected }) => [normal, selected]),
    camera_resources: [1040, 1041, 1042, 1043],
    visible_sets: [1, 2, 3, 4],
    route_points: navigationPoints.slice(0, 2),
    route_logs: routeLogs,
    target_object_ids: enemyObjectIds,
    target_logs: targetLogs,
    camera_logs: cameraLogs,
    provisional_point_coordinates: true,
  });
  return probes;
}

async function runScenarioOnce(server, executable, scenario, faction, viewport) {
  const id = scenarioId(scenario, faction, viewport);
  const folder = path.resolve(runDir, id);
  assert.ok(folder.startsWith(`${runDir}${path.sep}`), `scenario path escaped run directory: ${id}`);
  fs.mkdirSync(folder, { recursive: true });
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
      viewport: { width: viewport.width, height: viewport.height },
      deviceScaleFactor: viewport.device_scale_factor,
      locale: "en-US",
      timezoneId: "America/New_York",
      colorScheme: "dark",
      reducedMotion: "reduce",
      serviceWorkers: "block",
    });
    page = await context.newPage();
    if (scenario.battle_options_presentation) {
      // An occupied first save slot (the build's localStorage keys,
      // rebellion_data::save::slot_key / meta_key), so a battle's Game
      // Options can show its Load lit. The row needs a readable header
      // (FUN_00415cd0 reads it; an unreadable one leaves the row empty,
      // rebellion_data::save::game_options_saves); the body need not decode.
      const version = fs.readFileSync(path.join(root, "crates/rebellion-data/src/save.rs"), "utf8")
        .match(/pub const SAVE_VERSION: u32 = (\d+);/)[1];
      await page.addInitScript((v) => {
        localStorage.setItem(`rebellion_save_v${v}_0`, "AAAA");
        localStorage.setItem(`rebellion_meta_v${v}_0`, JSON.stringify({
          schema_version: 1,
          name: "Battle",
          game_tick: 1,
          state_fingerprint: { version: 1, value: "1" },
        }));
      }, version);
    }
    page.on("response", (response) => {
      if (new URL(response.url()).origin === `http://127.0.0.1:${server.address().port}`) {
        requests.push({ url: new URL(response.url()).pathname, status: response.status() });
      }
    });
    page.on("requestfailed", (request) => {
      // Escape on the trench run's movie stops its download (see below).
      // When it does so before the response arrives, the browser still
      // asked for the movie: count the request, with no status, so the
      // list check sees the movie either way.
      if (scenario.trench_run_outcome && /\/data\/video\/20[12]\.webm$/.test(request.url())
        && request.failure()?.errorText === "net::ERR_ABORTED") {
        const pathname = new URL(request.url()).pathname;
        if (!requests.some(({ url }) => url === pathname)) requests.push({ url: pathname, status: null, aborted: true });
        return;
      }
      errors.push(`requestfailed:${request.url()}:${request.failure()?.errorText}`);
    });
    page.on("pageerror", (error) => errors.push(`pageerror:${error.stack || error.message}`));
    page.on("console", (message) => {
      consoleLines.push({ type: message.type(), text: message.text() });
      if (message.type() === "error"
        || /missing.asset|\[bmp_cache\].*not found|\[tactical_3d\].*(unavailable|rejected)/i.test(message.text())) {
        errors.push(`console:${message.type()}:${message.text()}`);
      }
    });

    const code = fixtureCode(scenario, faction);
    const url = `http://127.0.0.1:${server.address().port}/?fixture-code=${code}`;
    await page.goto(url, { waitUntil: "load", timeout: 30_000 });
    if (scenario.trench_run_outcome) {
      // The trench run's movie (MDATA.201 / 202) plays before the battle
      // returns; Escape ends any movie (movie-flow.md, "Ported").
      const deadline = Date.now() + 30_000;
      while (!(await page.evaluate(() => Boolean(window.__openRebellionInterfaceReady?.status)))) {
        if (Date.now() > deadline) throw new Error("the trench run never returned to the battle");
        if (await page.evaluate(() => Boolean(document.querySelector("video")))) {
          await page.keyboard.press("Escape");
        }
        await page.waitForTimeout(250);
      }
    }
    await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status, null, {
      timeout: 30_000,
    });
    const ready = await page.evaluate(() => window.__openRebellionInterfaceReady);
    assert.equal(ready.status, battle ? "battle-ready" : "ready", JSON.stringify(ready));
    assert.equal(battle ? ready.fixture_code : ready.code, code);
    // The first frame the game shows once ready, before anything settles.
    const firstReadyBytes = battle ? null : await page.screenshot({ animations: "disabled" });
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(250);
    const stable = await stableFrame(page, folder);
    assert.equal(stable.first, stable.second, `unstable screenshot for ${id}`);
    const firstFrame = battle ? null : verifyFirstFrameComplete(viewport, firstReadyBytes, stable.bytes, folder);
    const messageIndexRail = battle ? null : verifyMessageIndexRail(faction, viewport, stable.bytes);
    const probes = battle
      ? [{ type: "production-tactical-entry", system: ready.system,
        attacker_ships: ready.attacker_ships, defender_ships: ready.defender_ships,
        fighters: ready.fighters }, ...(scenario.camera_journey
        ? await probeTacticalCameraJourney(page, viewport, folder, stable)
        : scenario.lod_journey
        ? await probeTacticalLodJourney(page, viewport, folder, stable)
        : scenario.slug === "battle-entry"
        ? await probeTactical(page, viewport, folder, stable)
        : scenario.fighter_detail_journey
        ? await probeTacticalFighterDetailJourney(
          page, viewport, folder, stable, consoleLines, ready,
        )
        : scenario.group_presentation
        ? await probeTacticalGroupPresentation(
          page, viewport, folder, stable, consoleLines, ready, faction,
        )
        : scenario.selected_navigation_presentation
        ? await probeTacticalSelectedNavigationPresentation(
          page, viewport, folder, stable, consoleLines, ready, faction,
        )
        : scenario.navigation_camera_presentation
        ? await probeTacticalNavigationCameraPresentation(
          page, viewport, folder, stable, consoleLines, ready,
        )
        : scenario.capital_panel_tip
        ? await probeTacticalCapitalPanelTip(page, viewport, folder, faction)
        : scenario.umgul_backdrop
        ? await probeTacticalUmgulBackdrop(page, viewport, folder, ready, faction)
        : scenario.coruscant_chase
        ? await probeTacticalCoruscantChase(page, viewport, folder, consoleLines, ready, faction)
        : scenario.battle_last_ship_lost
        ? await probeTacticalBattleLastShipLost(
          page, viewport, folder, consoleLines, ready, faction,
        )
        : scenario.battle_withdrawal_report
        ? await probeTacticalBattleWithdrawalReport(
          page, viewport, folder, consoleLines, ready, faction,
        )
        : scenario.battle_alert_retreat
        ? await probeTacticalBattleAlertRetreat(
          page, viewport, folder, consoleLines, ready, faction,
        )
        : scenario.battle_alert_presentation
        ? await probeTacticalBattleAlertEntry(
          page, viewport, folder, stable, consoleLines, ready, faction,
        )
        : scenario.tactical_audio_presentation
        ? probeTacticalAudioRouting(folder, stable, consoleLines, ready)
        : scenario.detail_escort_presentation
        ? await probeTacticalDetailEscortPresentation(
          page, viewport, folder, stable, consoleLines, ready,
        )
        : scenario.effect_presentation
        ? await probeTacticalEffectPresentation(folder, stable, consoleLines, ready)
        : scenario.projectile_field_presentation
        ? await probeTacticalProjectileFieldPresentation(folder, stable, consoleLines, ready)
        : scenario.subsystem_field_command_presentation
        ? probeTacticalSubsystemFieldCommandPresentation(viewport, folder, stable, ready)
        : scenario.live_subsystem_damage_presentation
        ? probeTacticalLiveSubsystemDamagePresentation(viewport, folder, stable, ready)
        : scenario.subsystem_repair_mobility_presentation
        ? probeTacticalSubsystemRepairMobilityPresentation(
          viewport, folder, stable, ready, consoleLines,
        )
        : scenario.maneuver_movement_presentation
        ? probeTacticalManeuverMovementPresentation(viewport, folder, stable, ready)
        : scenario.command_assignment_presentation
        ? await probeTacticalCommandAssignmentPresentation(
          page, viewport, folder, stable, consoleLines, ready,
        )
        : scenario.command_execution_presentation
        ? probeTacticalCommandExecutionPresentation(viewport, folder, stable, ready)
        : scenario.command_progression_presentation
        ? probeTacticalCommandProgressionPresentation(
          viewport, folder, stable, consoleLines, ready,
        )
        : scenario.attack_targeting_presentation
        ? probeTacticalAttackTargetingPresentation(viewport, folder, stable, ready, consoleLines)
        : scenario.attack_target_lost_presentation
        ? probeTacticalAttackTargetLostPresentation(viewport, folder, stable, ready, consoleLines)
        : scenario.attack_target_lifecycle_presentation
        ? probeTacticalAttackTargetLifecyclePresentation(viewport, folder, stable, ready)
        : scenario.trench_run_outcome
        ? probeTacticalTrenchRunOutcome(
          folder, stable, consoleLines, ready, scenario.trench_run_outcome,
        )
        : scenario.death_star_laser_journey
        ? await probeTacticalDeathStarLaserJourney(
          page, viewport, folder, stable, consoleLines, ready,
        )
        : scenario.death_star_presentation
        ? await probeTacticalDeathStarPresentation(
          page, viewport, folder, stable, consoleLines, ready,
        )
        : scenario.battle_results_presentation
        ? await probeTacticalBattleResultsPresentation(
          page, viewport, folder, stable, consoleLines, faction,
        )
        : scenario.battle_options_presentation
        ? await probeTacticalBattleOptionsPresentation(
          page, viewport, folder, stable, consoleLines, faction,
          Boolean(scenario.battle_options_withdrawal),
        )
        : scenario.selected_damage_presentation
        ? probeTacticalSelectedDamagePresentation(viewport, folder, stable, ready)
        : scenario.empty_space_presentation
        ? [{
            type: "empty-space-tactical-viewport",
            backdrop: verifyTacticalBackdrop(viewport, stable.bytes, ready.palette_resource_id),
            planet_loaded: false,
            planet_submitted: false,
            a0: compareEmptySpaceWithA0(viewport, stable.bytes, faction),
            screenshot_sha256: sha256(stable.bytes),
          }]
        : scenario.hull_scale_presentation
        ? probeTacticalHullScale(viewport, folder, stable, consoleLines, ready)
        : scenario.production_participants
        ? await probeTacticalProductionParticipants(
          page, viewport, folder, stable, consoleLines, faction, ready,
        )
        : scenario.tactical_proof
        ? [{ type: "tactical-lod-fixture", requested: ready.tactical_lod }]
        : [{ type: "tactical-3d-negative-control", proof_enabled: false }])]
      : await probeGid(page, faction, scenario, viewport, folder, consoleLines, ready);
    // Simulate Results from the alert never enters the tactical scene.
    const simulatedRoute = probes.some(({ type }) => type === "battle-alert-simulate-route");
    if (battle) {
      assert.equal(ready.schema_version, 36);
      assert.equal(ready.family, "tactical");
      assert.equal(ready.faction, faction);
      assert.equal(ready.proof_enabled, scenario.tactical_proof);
      assert.equal(ready.production_participants, Boolean(scenario.production_participants));
      assert.equal(ready.suppress_capital_fallback,
        Boolean(scenario.suppress_capital_fallback));
      assert.equal(ready.focus_player_fighter,
        Boolean(scenario.fighter_detail_journey || scenario.death_star_presentation));
      assert.equal(ready.group_presentation, Boolean(scenario.group_presentation));
      assert.equal(ready.effect_presentation, Boolean(scenario.effect_presentation));
      assert.equal(ready.projectile_field_presentation,
        Boolean(scenario.projectile_field_presentation));
      assert.equal(ready.selected_damage_presentation,
        Boolean(scenario.selected_damage_presentation));
      assert.equal(ready.selected_navigation_presentation,
        Boolean(scenario.selected_navigation_presentation));
      assert.equal(ready.navigation_camera_presentation,
        Boolean(scenario.navigation_camera_presentation));
      assert.equal(ready.empty_space_presentation,
        Boolean(scenario.empty_space_presentation));
      assert.equal(ready.detail_escort_presentation,
        Boolean(scenario.detail_escort_presentation));
      assert.equal(ready.battle_alert_presentation,
        Boolean(scenario.battle_alert_presentation));
      assert.equal(ready.battle_alert_retreat, Boolean(scenario.battle_alert_retreat));
      assert.equal(ready.battle_withdrawal_report, Boolean(scenario.battle_withdrawal_report));
      assert.equal(ready.battle_last_ship_lost, Boolean(scenario.battle_last_ship_lost));
      assert.equal(ready.umgul_backdrop, Boolean(scenario.umgul_backdrop));
      assert.equal(ready.coruscant_chase, Boolean(scenario.coruscant_chase));
      assert.equal(ready.tactical_audio_presentation,
        Boolean(scenario.tactical_audio_presentation));
      assert.equal(ready.hull_scale_presentation,
        Boolean(scenario.hull_scale_presentation));
      assert.equal(ready.subsystem_field_command_presentation,
        Boolean(scenario.subsystem_field_command_presentation));
      assert.equal(ready.live_subsystem_damage_presentation,
        Boolean(scenario.live_subsystem_damage_presentation));
      assert.equal(ready.subsystem_repair_mobility_presentation,
        Boolean(scenario.subsystem_repair_mobility_presentation));
      assert.equal(ready.maneuver_movement_presentation,
        Boolean(scenario.maneuver_movement_presentation));
      assert.equal(ready.command_assignment_presentation,
        Boolean(scenario.command_assignment_presentation));
      assert.equal(ready.command_execution_presentation,
        Boolean(scenario.command_execution_presentation));
      assert.equal(ready.command_progression_presentation,
        Boolean(scenario.command_progression_presentation));
      assert.equal(ready.attack_targeting_presentation,
        Boolean(scenario.attack_targeting_presentation));
      assert.equal(ready.attack_target_lifecycle_presentation,
        Boolean(scenario.attack_target_lifecycle_presentation));
      assert.equal(ready.attack_target_lost_presentation,
        Boolean(scenario.attack_target_lost_presentation));
      assert.equal(ready.death_star_presentation,
        Boolean(scenario.death_star_presentation));
      assert.equal(ready.battle_results_presentation,
        Boolean(scenario.battle_results_presentation));
      assert.equal(ready.battle_options_presentation,
        Boolean(scenario.battle_options_presentation));
      assert.equal(ready.battle_options_withdrawal,
        Boolean(scenario.battle_options_withdrawal));
      assert.equal(ready.death_star_laser_journey,
        Boolean(scenario.death_star_laser_journey));
      assert.equal(ready.trench_run_outcome, scenario.trench_run_outcome ?? null);
      assert.equal(ready.death_star === null, !scenario.death_star_presentation);
      assert.ok(Number.isSafeInteger(ready.system_picture_id)
        && ready.system_picture_id >= 1 && ready.system_picture_id <= 27,
      "tactical fixture lacks its SYSTEMSD picture identity");
      assert.equal(ready.planet_resource_id, ready.system_picture_id + 5500);
      assert.equal(ready.palette_resource_id, ready.system_picture_id + 5530);
      assert.ok(ready.attacker_ships > 0 && ready.defender_ships > 0);
      // A0 TAC-01-C003's battle has no fighters.
      // The retreat fixture brings no fighters either.
      assert.equal(ready.fighters > 0,
        !scenario.hull_scale_presentation && !scenario.battle_alert_retreat
          && !scenario.battle_withdrawal_report);
      const groupLayout = faction === "alliance"
        ? { first_active_objects: 12, second_active_objects: 2 }
        : { first_active_objects: 2, second_active_objects: 12 };
      const effectLayout = faction === "alliance"
        ? { first_active_objects: 7, second_active_objects: 2 }
        : { first_active_objects: 2, second_active_objects: 7 };
      const projectileFieldLayout = { first_active_objects: 4, second_active_objects: 4 };
      assert.deepEqual(ready.source_layout, scenario.coruscant_chase ? {
        // A0 TAC-04-C014: the carrier and two squadrons against one hull,
        // 100 + 3 x 3 (FUN_005ab650).
        first_active_objects: 3,
        second_active_objects: 1,
        battle_extent: 109,
        outer_positive_z: 54.5,
        outer_negative_z: -54.5,
        inner_negative_z: -34.5,
        inner_positive_z: 34.5,
      } : scenario.battle_alert_retreat ? {
        // The viewer's one hull against the enemy's four: 100 + 3 x 4.
        first_active_objects: faction === "alliance" ? 1 : 4,
        second_active_objects: faction === "alliance" ? 4 : 1,
        battle_extent: 112,
        outer_positive_z: 56,
        outer_negative_z: -56,
        inner_negative_z: -36,
        inner_positive_z: 36,
      } : scenario.hull_scale_presentation || scenario.battle_withdrawal_report ? {
        // One unit a side (A0 TAC-01-C003): 100 + 3 (FUN_005ab650).
        first_active_objects: 1,
        second_active_objects: 1,
        battle_extent: 103,
        outer_positive_z: 51.5,
        outer_negative_z: -51.5,
        inner_negative_z: -31.5,
        inner_positive_z: 31.5,
      } : scenario.group_presentation ? {
        ...groupLayout,
        battle_extent: 136,
        outer_positive_z: 68,
        outer_negative_z: -68,
        inner_negative_z: -48,
        inner_positive_z: 48,
      } : scenario.effect_presentation ? {
        ...effectLayout,
        battle_extent: 121,
        outer_positive_z: 60.5,
        outer_negative_z: -60.5,
        inner_negative_z: -40.5,
        inner_positive_z: 40.5,
      } : scenario.projectile_field_presentation
        || scenario.detail_escort_presentation
        || scenario.subsystem_field_command_presentation
        || scenario.live_subsystem_damage_presentation
        || scenario.subsystem_repair_mobility_presentation
        || scenario.maneuver_movement_presentation
        || scenario.command_execution_presentation
        || scenario.selected_navigation_presentation
        || scenario.navigation_camera_presentation ? {
        ...projectileFieldLayout,
        battle_extent: 112,
        outer_positive_z: 56,
        outer_negative_z: -56,
        inner_negative_z: -36,
        inner_positive_z: 36,
      } : scenario.command_progression_presentation
        || scenario.attack_targeting_presentation
        || scenario.attack_target_lifecycle_presentation ? {
        first_active_objects: 5,
        second_active_objects: 5,
        battle_extent: 115,
        outer_positive_z: 57.5,
        outer_negative_z: -57.5,
        inner_negative_z: -37.5,
        inner_positive_z: 37.5,
      } : scenario.command_assignment_presentation
        || scenario.battle_options_withdrawal ? {
        first_active_objects: 3,
        second_active_objects: 3,
        battle_extent: 109,
        outer_positive_z: 54.5,
        outer_negative_z: -54.5,
        inner_negative_z: -34.5,
        inner_positive_z: 34.5,
      } : scenario.battle_alert_presentation ? {
        // A ship and five squadrons a side (the Battle Alert's forces list
        // outgrows its height and scrolls).
        first_active_objects: 6,
        second_active_objects: 6,
        battle_extent: 118,
        outer_positive_z: 59,
        outer_negative_z: -59,
        inner_negative_z: -39,
        inner_positive_z: 39,
      } : scenario.battle_results_presentation ? {
        // Three ships and five squadrons a side (the Fighters list
        // outgrows its height and scrolls).
        first_active_objects: 8,
        second_active_objects: 8,
        battle_extent: 124,
        outer_positive_z: 62,
        outer_negative_z: -62,
        inner_negative_z: -42,
        inner_positive_z: 42,
      } : scenario.death_star_presentation ? {
        // The Imperial fleet brings no fighter group: FUN_005ad930 enables
        // Attack Death Star only while fewer Imperial than Alliance groups
        // are on the field.
        first_active_objects: 2,
        second_active_objects: 1,
        battle_extent: 106,
        outer_positive_z: 53,
        outer_negative_z: -53,
        inner_negative_z: -33,
        inner_positive_z: 33,
      } : {
        first_active_objects: 2,
        second_active_objects: 2,
        battle_extent: 106,
        outer_positive_z: 53,
        outer_negative_z: -53,
        inner_negative_z: -33,
        inner_positive_z: 33,
      });
      assert.equal(ready.participants.length,
        scenario.coruscant_chase ? 10
        : scenario.group_presentation ? 14
          : scenario.effect_presentation ? 9
          : scenario.projectile_field_presentation
            || scenario.detail_escort_presentation
            || scenario.subsystem_field_command_presentation
            || scenario.live_subsystem_damage_presentation
            || scenario.subsystem_repair_mobility_presentation
            || scenario.maneuver_movement_presentation
            || scenario.command_execution_presentation
            || scenario.selected_navigation_presentation
            || scenario.navigation_camera_presentation ? 8
            : scenario.command_progression_presentation
              || scenario.attack_targeting_presentation
              || scenario.attack_target_lifecycle_presentation ? 10
              : scenario.command_assignment_presentation
                || scenario.battle_options_withdrawal ? 6
                : scenario.battle_results_presentation ? 16
                  : scenario.battle_alert_presentation ? 12
                  : scenario.battle_alert_retreat ? 5
                  : scenario.death_star_presentation ? 3
                    : scenario.hull_scale_presentation
                      || scenario.battle_withdrawal_report ? 2 : 4);
      const expectedParticipantLanes = new Map([
        ["capital-ship:alliance", -53],
        ["capital-ship:empire", 53],
        ["fighter-group:alliance", -33],
        ["fighter-group:empire", 33],
      ]);
      const expectedParticipantResources = new Map([
        ["capital-ship:alliance", {
          datId: 64, ordinal: 0, base: 2010, opposingBase: null,
          close: null, far: null, indicator: null,
        }],
        ["capital-ship:empire", {
          datId: 128, ordinal: 15, base: 2510, opposingBase: null,
          close: null, far: null, indicator: null,
        }],
        ["fighter-group:alliance", {
          datId: 1, ordinal: 31, base: 4020, opposingBase: 4024,
          close: 4024, far: 4029, indicator: 4204,
        }],
        ["fighter-group:empire", {
          datId: 5, ordinal: 33, base: 4100, opposingBase: 4104,
          close: 4104, far: 4109, indicator: 4204,
        }],
      ]);
      if (scenario.group_presentation) {
        const playerShips = ready.participants.filter((participant) =>
          participant.kind === "capital-ship" && participant.faction === faction);
        const opponentShips = ready.participants.filter((participant) =>
          participant.kind === "capital-ship" && participant.faction !== faction);
        const playerFighters = ready.participants.filter((participant) =>
          participant.kind === "fighter-group" && participant.faction === faction);
        const opponentFighters = ready.participants.filter((participant) =>
          participant.kind === "fighter-group" && participant.faction !== faction);
        assert.deepEqual(playerShips.map(({ fleet_roster_index }) => fleet_roster_index),
          [0, 1, 2, 3, 4, 5, 6, 7]);
        assert.deepEqual(playerShips.map(({ task_force }) => task_force),
          [0, 1, 2, 3, 4, 5, 6, 7]);
        assert.equal(opponentShips.length, 1);
        assert.deepEqual(playerFighters.map(({ fleet_roster_index }) => fleet_roster_index),
          [0, 1, 2, 3]);
        assert.deepEqual(playerFighters.map(({ fighter_group }) => fighter_group), [0, 1, 2, 3]);
        assert.equal(opponentFighters.length, 1);
        const playerBase = faction === "alliance" ? 4020 : 4100;
        for (const fighter of playerFighters) {
          assert.equal(fighter.active_close_resource, playerBase + fighter.fighter_group);
          assert.equal(fighter.active_far_resource, playerBase + fighter.fighter_group + 5);
          assert.equal(fighter.active_indicator_resource, 4200 + fighter.fighter_group);
        }
        assert.equal(opponentFighters[0].active_close_resource,
          faction === "alliance" ? 4104 : 4024);
        assert.equal(opponentFighters[0].active_far_resource,
          faction === "alliance" ? 4109 : 4029);
        assert.equal(opponentFighters[0].active_indicator_resource, 4204);
        const playerShipZ = faction === "alliance" ? -68 : 68;
        const playerFighterZ = faction === "alliance" ? -48 : 48;
        assert.ok(playerShips.every(({ source_position }) => source_position[2] === playerShipZ));
        assert.ok(playerFighters.every(({ source_position }) =>
          source_position[2] === playerFighterZ));
      } else if (scenario.effect_presentation) {
        const playerShips = ready.participants.filter((participant) =>
          participant.kind === "capital-ship" && participant.faction === faction);
        const opponentShips = ready.participants.filter((participant) =>
          participant.kind === "capital-ship" && participant.faction !== faction);
        const playerFighters = ready.participants.filter((participant) =>
          participant.kind === "fighter-group" && participant.faction === faction);
        const opponentFighters = ready.participants.filter((participant) =>
          participant.kind === "fighter-group" && participant.faction !== faction);
        assert.deepEqual(playerShips.map(({ fleet_roster_index }) => fleet_roster_index),
          [0, 1, 2, 3, 4, 5]);
        assert.equal(opponentShips.length, 1);
        assert.equal(playerFighters.length, 1);
        assert.equal(opponentFighters.length, 1);
        const playerShipZ = faction === "alliance" ? -60.5 : 60.5;
        const playerFighterZ = faction === "alliance" ? -40.5 : 40.5;
        assert.ok(playerShips.every(({ source_position: position }) => position[2] === playerShipZ));
        assert.ok(playerFighters.every(({ source_position: position }) =>
          position[2] === playerFighterZ));
      } else if (scenario.projectile_field_presentation
        || scenario.detail_escort_presentation
        || scenario.subsystem_field_command_presentation
        || scenario.live_subsystem_damage_presentation
        || scenario.subsystem_repair_mobility_presentation
        || scenario.maneuver_movement_presentation
        || scenario.command_execution_presentation
        || scenario.selected_navigation_presentation
        || scenario.navigation_camera_presentation
        || scenario.command_progression_presentation
        || scenario.attack_targeting_presentation
        || scenario.attack_target_lifecycle_presentation) {
        const allianceShips = ready.participants.filter((participant) =>
          participant.kind === "capital-ship" && participant.faction === "alliance");
        const empireShips = ready.participants.filter((participant) =>
          participant.kind === "capital-ship" && participant.faction === "empire");
        const allianceFighters = ready.participants.filter((participant) =>
          participant.kind === "fighter-group" && participant.faction === "alliance");
        const empireFighters = ready.participants.filter((participant) =>
          participant.kind === "fighter-group" && participant.faction === "empire");
        assert.deepEqual(allianceShips.map(({ fleet_roster_index }) => fleet_roster_index),
          [0, 1, 2]);
        assert.deepEqual(empireShips.map(({ fleet_roster_index }) => fleet_roster_index),
          [0, 1, 2]);
        const expectedFighters = scenario.command_progression_presentation
          || scenario.attack_targeting_presentation
          || scenario.attack_target_lifecycle_presentation ? 2 : 1;
        assert.equal(allianceFighters.length, expectedFighters);
        assert.equal(empireFighters.length, expectedFighters);
        if (scenario.command_progression_presentation
          || scenario.attack_targeting_presentation
          || scenario.attack_target_lifecycle_presentation) {
          assert.ok(allianceShips.every(({ fleet_roster_index }) =>
            [0, 1, 2].includes(fleet_roster_index)));
          assert.ok(empireShips.every(({ fleet_roster_index }) =>
            [0, 1, 2].includes(fleet_roster_index)));
        } else {
          // The maneuver fixture moves its selected ship (roster 0) alone:
          // its force's anchor steps 0.95 x 78.75 x 0.25 (0x005eb1a0) and the
          // ship snaps onto it (FUN_005b2f30); every other ship stands.
          const movedFaction = scenario.maneuver_movement_presentation ? faction : null;
          const expectedZ = (shipFaction, rosterIndex, restZ) =>
            movedFaction === shipFaction && rosterIndex === 0
              ? restZ - Math.sign(restZ) * 0.95 * 78.75 * 0.25
              : restZ;
          assert.ok(allianceShips.filter(({ active }) => active).every(
            ({ source_position: position, fleet_roster_index }) =>
              Math.abs(position[2] - expectedZ("alliance", fleet_roster_index, -56)) < 0.0001),
          "Alliance formation did not follow its anchor");
          assert.ok(empireShips.filter(({ active }) => active).every(
            ({ source_position: position, fleet_roster_index }) =>
              Math.abs(position[2] - expectedZ("empire", fleet_roster_index, 56)) < 0.0001),
          "Imperial formation did not follow its anchor");
          assert.equal(allianceFighters[0].source_position[2], -36);
          assert.equal(empireFighters[0].source_position[2], 36);
        }
      } else if (scenario.death_star_presentation
        || scenario.battle_options_presentation
        || scenario.command_assignment_presentation) {
        const allianceShips = ready.participants.filter((participant) =>
          participant.kind === "capital-ship" && participant.faction === "alliance");
        const empireShips = ready.participants.filter((participant) =>
          participant.kind === "capital-ship" && participant.faction === "empire");
        const allianceFighters = ready.participants.filter((participant) =>
          participant.kind === "fighter-group" && participant.faction === "alliance");
        const empireFighters = ready.participants.filter((participant) =>
          participant.kind === "fighter-group" && participant.faction === "empire");
        const expectedShips = scenario.battle_options_withdrawal ? 2 : 1;
        assert.equal(allianceShips.length, expectedShips);
        assert.equal(empireShips.length, expectedShips);
        const expectedFighters = scenario.command_assignment_presentation ? 2 : 1;
        assert.equal(allianceFighters.length, expectedFighters);
        // A Death Star battle brings no Imperial fighter group (FUN_005ad930
        // needs fewer Imperial than Alliance groups for Attack Death Star).
        assert.equal(empireFighters.length,
          scenario.death_star_presentation ? 0 : expectedFighters);
        const allianceShipZ = scenario.death_star_laser_journey && faction === "empire"
          ? ready.source_layout.outer_positive_z
          : ready.source_layout.outer_negative_z;
        assert.ok(allianceShips.every(({ source_position: position }) =>
          position[2] === allianceShipZ));
        assert.ok(empireShips.every(({ source_position: position }) =>
          position[2] === ready.source_layout.outer_positive_z));
        assert.ok(allianceFighters.every(({ source_position: position }) =>
          position[2] === ready.source_layout.inner_negative_z));
        assert.ok(empireFighters.every(({ source_position: position }) =>
          position[2] === ready.source_layout.inner_positive_z));
      } else if (scenario.battle_results_presentation) {
        const allianceShips = ready.participants.filter((participant) =>
          participant.kind === "capital-ship" && participant.faction === "alliance");
        const empireShips = ready.participants.filter((participant) =>
          participant.kind === "capital-ship" && participant.faction === "empire");
        const allianceFighters = ready.participants.filter((participant) =>
          participant.kind === "fighter-group" && participant.faction === "alliance");
        const empireFighters = ready.participants.filter((participant) =>
          participant.kind === "fighter-group" && participant.faction === "empire");
        assert.equal(allianceShips.length, 3);
        assert.equal(empireShips.length, 3);
        assert.equal(allianceFighters.length, 5);
        assert.equal(empireFighters.length, 5);
        assert.ok(allianceShips.every(({ source_position: position }) =>
          position[2] === ready.source_layout.outer_negative_z));
        assert.ok(empireShips.every(({ source_position: position }) =>
          position[2] === ready.source_layout.outer_positive_z));
        assert.ok(allianceFighters.every(({ source_position: position }) =>
          position[2] === ready.source_layout.inner_negative_z));
        assert.ok(empireFighters.every(({ source_position: position }) =>
          position[2] === ready.source_layout.inner_positive_z));
      } else if (scenario.battle_alert_presentation) {
        // A ship and five squadrons a side, on their side's lanes.
        const of = (kind, faction) => ready.participants.filter((participant) =>
          participant.kind === kind && participant.faction === faction);
        assert.equal(of("capital-ship", "alliance").length, 1);
        assert.equal(of("capital-ship", "empire").length, 1);
        assert.equal(of("fighter-group", "alliance").length, 5);
        assert.equal(of("fighter-group", "empire").length, 5);
        assert.ok(of("capital-ship", "alliance").every(({ source_position: position }) =>
          position[2] === ready.source_layout.outer_negative_z));
        assert.ok(of("capital-ship", "empire").every(({ source_position: position }) =>
          position[2] === ready.source_layout.outer_positive_z));
        assert.ok(of("fighter-group", "alliance").every(({ source_position: position }) =>
          position[2] === ready.source_layout.inner_negative_z));
        assert.ok(of("fighter-group", "empire").every(({ source_position: position }) =>
          position[2] === ready.source_layout.inner_positive_z));
      } else if (scenario.battle_withdrawal_report) {
        assert.deepEqual(ready.participants.map(({ kind, faction: side }) => [kind, side]),
          [["capital-ship", "alliance"], ["capital-ship", "empire"]]);
      } else if (scenario.battle_alert_retreat) {
        const of = (kind, side) => ready.participants.filter((participant) =>
          participant.kind === kind && participant.faction === side);
        assert.equal(of("capital-ship", faction).length, 1);
        assert.equal(of("capital-ship", faction === "alliance" ? "empire" : "alliance").length, 4);
        assert.equal(of("fighter-group", "alliance").length + of("fighter-group", "empire").length, 0);
      } else if (scenario.coruscant_chase) {
        // A0 TAC-04-C014: the Alliance Escort Carrier (2050) and its X-wing
        // and Y-wing squadrons against Coruscant's Imperial Star Destroyer
        // (2560), each on its side's lane.
        // The home fleet's six TIE squadrons are listed but not active:
        // the layout counts one Imperial object.
        const inactive = ready.participants.filter(({ active }) => !active);
        assert.deepEqual(inactive.map(({ kind, faction }) => [kind, faction]),
          Array(6).fill(["fighter-group", "empire"]));
        assert.deepEqual(ready.participants.filter(({ active }) => active)
          .map(({ kind, faction, resource_base, source_position }) =>
            [kind, faction, kind === "capital-ship" ? resource_base : null, source_position[2]]), [
          ["capital-ship", "alliance", 2050, -54.5],
          ["capital-ship", "empire", 2560, 54.5],
          ["fighter-group", "alliance", null, -34.5],
          ["fighter-group", "alliance", null, -34.5],
        ]);
      } else if (scenario.hull_scale_presentation) {
        // A0 TAC-01-C003: the Medium Transport (2070) and the Victory
        // Destroyer (2550), each alone on its side's capital lane.
        assert.deepEqual(ready.participants.map(({ kind, faction, resource_base, source_position }) =>
          [kind, faction, resource_base, source_position]), [
          ["capital-ship", "alliance", 2070, [-0, 0, -51.5]],
          ["capital-ship", "empire", 2550, [-0, 0, 51.5]],
        ]);
      } else {
        for (const participant of ready.participants) {
          const key = `${participant.kind}:${participant.faction}`;
          assert.ok(expectedParticipantLanes.has(key), `unexpected participant ${key}`);
          const expectedResource = expectedParticipantResources.get(key);
          assert.equal(participant.class_dat_id, expectedResource.datId,
            `${key} has the wrong stable DAT identity`);
          assert.equal(participant.fleet_roster_index, 0);
          assert.equal(participant.tactical_ordinal, expectedResource.ordinal,
            `${key} has the wrong tactical ordinal`);
          assert.equal(participant.resource_base, expectedResource.base,
            `${key} has the wrong tactical resource base`);
          assert.equal(participant.opposing_resource_base, expectedResource.opposingBase,
            `${key} has the wrong opposing tactical resource base`);
          assert.equal(participant.initial_close_resource, expectedResource.close,
            `${key} has the wrong initial close resource`);
          assert.equal(participant.initial_far_resource, expectedResource.far,
            `${key} has the wrong initial far resource`);
          assert.equal(participant.initial_indicator_resource, expectedResource.indicator,
            `${key} has the wrong initial indicator resource`);
          assert.deepEqual(participant.source_position,
            [-0, 0, expectedParticipantLanes.get(key)], `${key} has the wrong source position`);
        }
      }
      probes.push({
        type: "source-bound-tactical-participants",
        executable_functions: ["FUN_005ab650", "FUN_005a9030"],
        layout: ready.source_layout,
        participants: ready.participants,
      });
      const packLog = consoleLines.find(({ text }) => text.includes("runtime_asset_pack loaded"));
      assert.match(packLog?.text || "", /tactical_meshes=87 tactical_textures=397/,
        "runtime pack did not install the complete tactical object corpus");
      const familyLogs = consoleLines.filter(({ text }) =>
        text.includes("[tactical_3d] family_loaded base=2560"));
      const lodLogs = consoleLines.filter(({ text }) =>
        text.includes("[tactical_3d] lod_selection"));
      const cameraLogs = consoleLines.filter(({ text }) =>
        text.includes("[tactical_3d] camera_source"));
      const layoutLogs = consoleLines.filter(({ text }) =>
        text.includes("[tactical_3d] layout_source"));
      const participantFamilyLogs = consoleLines.filter(({ text }) =>
        text.includes("[tactical_3d] participant_family_loaded"));
      const participantSceneLogs = consoleLines.filter(({ text }) =>
        text.includes("[tactical_3d] participant_scene"));
      const participantBoundsLogs = consoleLines.filter(({ text }) =>
        text.includes("[tactical_3d] participant_bounds"));
      const fighterFamilyLogs = consoleLines.filter(({ text }) =>
        text.includes("[tactical_3d] fighter_family_loaded"));
      const fighterSceneLogs = consoleLines.filter(({ text }) =>
        text.includes("[tactical_3d] fighter_scene"));
      const planetLoadLogs = consoleLines.filter(({ text }) =>
        text.includes("[tactical_3d] planet_loaded"));
      const planetSceneLogs = consoleLines.filter(({ text }) =>
        text.includes("[tactical_3d] planet_scene"));
      const expectsPlanet = Boolean(
        scenario.production_participants || scenario.suppress_capital_fallback,
      ) && !scenario.battle_results_presentation && !scenario.empty_space_presentation
        && !simulatedRoute;
      assert.equal(planetLoadLogs.length, expectsPlanet ? 1 : 0,
        "tactical scene loaded the wrong number of system-selected planets");
      assert.equal(planetSceneLogs.length, expectsPlanet ? 1 : 0,
        "tactical scene submitted the wrong number of system-selected planets");
      if (expectsPlanet) {
        assert.match(planetLoadLogs[0]?.text || "",
          new RegExp(`resource=${ready.planet_resource_id} `
            + `palette_resource_id=${ready.palette_resource_id} dimensions=256x256 `
            + "texture_filter=nearest alpha=black_transparent "
            + "source=FUN_0059a850,FUN_00596ad0,FUN_005c2e60,FUN_005d4d10"),
        "tactical planet load used the wrong resource, palette, or source state");
        assert.match(planetSceneLogs[0]?.text || "",
          new RegExp(`resource=${ready.planet_resource_id} `
            + `palette_resource_id=${ready.palette_resource_id} dimensions=256x256 `
            + "visible=(false|true center=(-?[0-9.]+),(-?[0-9.]+) size=([0-9.]+)) "
            + "source=FUN_0059a850,FUN_00596ad0,FUN_005c2e60,FUN_005d4d10"),
        "tactical planet scene used the wrong resource or placement");
        // FUN_005d4d10 hangs the planet at +2e x, outside the first view
        // (A0 TAC-01-C002/C003 show no planet until the camera turns). The
        // port draws the 256x256 decal when its square reaches the view, but
        // the planet is the disc inside it (transparent black corners), so
        // the disc must stay outside the scene viewport (17, 30, 440, 438
        // at 640x480, tactical_scene_viewport). Fixture 45's corrected
        // Stand Off places some fleets so the square's corner touches it.
        const shown = (planetSceneLogs[0]?.text || "")
          .match(/visible=true center=(-?[0-9.]+),(-?[0-9.]+) size=([0-9.]+)/);
        if (shown) {
          const s = Math.min(viewport.width / 640, viewport.height / 480);
          const ox = (viewport.width - 640 * s) / 2;
          const oy = (viewport.height - 480 * s) / 2;
          const rect = { x: ox + 17 * s, y: oy + 30 * s, w: 440 * s, h: 438 * s };
          const [cx, cy, size] = shown.slice(1).map(Number);
          const nx = Math.max(rect.x, Math.min(cx, rect.x + rect.w));
          const ny = Math.max(rect.y, Math.min(cy, rect.y + rect.h));
          assert.ok(Math.hypot(cx - nx, cy - ny) >= size / 2,
            `tactical planet disc reaches the first view (center ${cx},${cy} size ${size})`);
        }
        probes.push({
          type: "system-selected-tactical-planet",
          executable_functions: [
            "FUN_0040b0e0", "FUN_00509610", "FUN_00595d60", "FUN_0059a850",
            "FUN_00596ad0", "FUN_005c2e60",
          ],
          system_picture_id: ready.system_picture_id,
          planet_resource_id: ready.planet_resource_id,
          palette_resource_id: ready.palette_resource_id,
          dimensions: [256, 256],
          texture_filter: "nearest",
          placement_acceptance: "provisional-A1-only",
        });
      }
      if (scenario.tactical_proof) {
        assert.equal(familyLogs.length, 1,
          "source-bound tactical LOD family did not emit exactly one load event");
        assert.match(familyLogs[0]?.text || "",
          new RegExp(`resources=2560,2561,2562 textures=SDESTI52\\.BMP,SDESTI_M\\.BMP `
            + `palette_selector=${ready.system_picture_id} `
            + `palette_resource_id=${ready.palette_resource_id} palette_flags=68 `
            + `transform=authored_xyz_z_reflection .*family_loads=1`),
          "source-bound tactical LOD family did not load exactly once");
        assert.match(familyLogs[0]?.text || "",
          /light_directional_rgb=0\.8 light_ambient_rgb=0\.5 light_frame_source=5,5,-1 surface_to_light_rh=0\.70014006,0\.70014006,0\.14002801 light_constraint=z/,
          "source-traced tactical light rig did not preserve its retained-mode contract");
        assert.match(familyLogs[0]?.text || "",
          /render_quality=flat device_dither=false texture_filter=nearest mip_filter=none source_cull=d3dcull_ccw target_cull=back_ccw depth_test=less_equal depth_write=true specular=true material=ambient_diffuse_specular_emissive/,
          "source-traced tactical renderer did not preserve its retained-mode device state");
        const expectedResources = scenario.expected_lod_sequence
          || [scenario.expected_lod_resource];
        assert.equal(lodLogs.length, expectedResources.length,
          "source-derived tactical LOD fixture emitted the wrong selection-event count");
        const selectedResources = lodLogs.map(({ text }) => {
          assert.match(text, /family_loads=1/,
            "tactical LOD selection observed a reloaded family");
          const match = text.match(/resource_id=(\d+)/);
          assert.ok(match, "tactical LOD selection omitted its resource identity");
          return Number(match[1]);
        });
        assert.deepEqual(selectedResources, expectedResources,
          "source-derived tactical LOD selection chose the wrong resource sequence");
        probes.push({
          type: "source-bound-tactical-3d-lod-family",
          meshes: ["2560/1033", "2561/1033", "2562/1033"],
          textures: ["SDESTI52.BMP/1033", "SDESTI_M.BMP/1033"],
          selected_resources: selectedResources,
          palette: {
            system_picture_id: ready.system_picture_id,
            resource_id: ready.palette_resource_id,
            flags: 68,
          },
          transform: "authored_xyz_z_reflection",
          family_loads: 1,
        });
        probes.push({
          type: "source-traced-tactical-light-rig",
          executable_function: "FUN_005d4d10",
          direct3drm: {
            directional: {
              type: 3,
              rgb: [0.8, 0.8, 0.8],
              frame_source: [5, 5, -1],
              target_source: [0, 0, 0],
              constraint: "z",
            },
            ambient: {
              type: 0,
              rgb: [0.5, 0.5, 0.5],
            },
          },
          surface_to_light_rh: [0.70014006, 0.70014006, 0.14002801],
        });
        probes.push({
          type: "source-traced-tactical-render-state",
          executable_functions: ["FUN_005c1c10", "FUN_005d6e10"],
          direct3drm: {
            dither: false,
            render_quality: "flat",
            texture_filter: "nearest",
            mip_filter: "none",
            source_cull: "d3dcull_ccw",
            target_cull: "back_cw",
            depth_test: "less_equal",
            depth_write: true,
            specular: true,
            material: "ambient_diffuse_specular_emissive",
          },
        });
        if (scenario.camera_journey) {
          assert.equal(layoutLogs.length, 1,
            "source camera journey emitted the wrong layout-event count");
          assert.match(layoutLogs[0]?.text || "",
            /first_active_objects=2 second_active_objects=2 battle_extent=106 outer_positive_z=53 outer_negative_z=-53 inner_negative_z=-33 inner_positive_z=33 source=FUN_005ab650/,
            "source camera journey did not use the fixture force composition");
          assert.equal(cameraLogs.length, 8,
            "source camera journey emitted the wrong state count");
          const states = cameraLogs.map(({ text }) => {
            const value = (label) => {
              const match = text.match(new RegExp(`${label}=(-?[0-9.]+)`));
              assert.ok(match, `source camera log omitted ${label}`);
              return Number(match[1]);
            };
            assert.match(text, /handedness=lh_y_up_to_rh_y_up/);
            return {
              pitch: value("pitch"),
              yaw: value("yaw"),
              field: value("field"),
              zoom_step: value("zoom_step"),
              orbit_step: value("orbit_step"),
              distance: value("distance"),
              near: value("near"),
              far: value("far"),
              fovy_radians: value("fovy_radians"),
              target_object_id: value("target_object_id"),
              target_x: value("target_x"),
              target_y: value("target_y"),
              target_z: value("target_z"),
            };
          });
          const initialYaw = faction === "alliance" ? -30 : 150;
          assert.deepEqual(states.map(({ pitch }) => pitch), [30, 30, 30, 30, 30, 35, 30, 30]);
          assert.deepEqual(states.map(({ yaw }) => yaw),
            [initialYaw, initialYaw, initialYaw, initialYaw - 5, initialYaw, initialYaw, initialYaw,
              initialYaw]);
          assert.deepEqual(states.map(({ zoom_step }) => zoom_step), [5, 4, 5, 5, 5, 5, 5, 5]);
          assert.deepEqual(states.map(({ orbit_step }) => orbit_step), [5, 4, 5, 5, 5, 5, 5, 5]);
          assert.ok(Math.abs(states[0].field - 0.2) < 1e-6);
          assert.ok(Math.abs(states[1].field - 0.18) < 1e-6);
          assert.ok(Math.abs(states[2].field - 0.198) < 1e-6);
          assert.ok(states.every(({ distance, near, far }) =>
            Math.abs(distance - 180.2) < 1e-4 && near === 1 && Math.abs(far - 450.5) < 1e-4));
          const selectedObjectId = faction === "alliance" ? 1 : 2;
          assert.deepEqual(states.map(({ target_object_id }) => target_object_id),
            [0, 0, 0, 0, 0, 0, 0, selectedObjectId]);
          assert.ok(states.slice(0, -1).every(({ target_x, target_y, target_z }) =>
            target_x === 0 && target_y === 0 && target_z === 0));
          assert.equal(states.at(-1).target_x, -0);
          assert.equal(states.at(-1).target_y, 0);
          assert.equal(states.at(-1).target_z, faction === "alliance" ? 53 : -53);
          probes.push({
            type: "source-traced-tactical-camera-journey",
            executable_functions: [
              "FUN_005d9490", "FUN_005d9620", "FUN_005d9640", "FUN_00595be0",
              "FUN_005c1080", "0x005d97c0",
            ],
            states,
          });
          probes.push({
            type: "source-traced-tactical-layout",
            executable_function: "FUN_005ab650",
            active_objects: { first_force: 2, second_force: 2 },
            battle_extent: 106,
            z_lanes: [53, -53, -33, 33],
          });
        } else {
          assert.equal(cameraLogs.length, 0,
            "non-camera tactical proof unexpectedly enabled the source camera");
          assert.equal(layoutLogs.length, 0,
            "non-camera tactical proof unexpectedly enabled the source layout");
        }
      } else {
        assert.equal(familyLogs.length, 0, "negative control loaded the tactical LOD family");
        assert.equal(lodLogs.length, 0, "negative control submitted a tactical LOD mesh");
        if (scenario.production_participants) {
          if (!scenario.battle_results_presentation && !scenario.tactical_audio_presentation
            && !simulatedRoute) {
          // A0 TAC-01-C003's battle has a Medium Transport and a Victory.
          // The contents fixture takes each side's first class with room for
          // three regiments (CAPSHPSD 7 and 20).
          const participantFamilies = scenario.hull_scale_presentation ? [2070, 2550]
            : scenario.detail_escort_presentation ? [2080, 2560]
            // A0 TAC-04-C014: the Escort Carrier and the Star Destroyer.
            : scenario.coruscant_chase ? [2050, 2560] : [2010, 2510];
          assert.equal(participantFamilyLogs.length, 2,
            "production participants did not load exactly two source families");
          assert.deepEqual(participantFamilyLogs.map(({ text }) => {
            const match = text.match(/base=(\d+)/);
            assert.ok(match, "participant family log omitted its base");
            return Number(match[1]);
          }).sort((a, b) => a - b), participantFamilies);
          assert.equal(participantSceneLogs.length, 1,
            "production participants did not emit one scene submission");
          const expectedShipCount = scenario.detail_escort_presentation
            ? ready.participants.filter(({ kind, active }) =>
              kind === "capital-ship" && active).length
            : ready.attacker_ships + ready.defender_ships;
          assert.match(participantSceneLogs[0].text,
            new RegExp(`requested=${expectedShipCount} rendered=${expectedShipCount} `
              + `families=${participantFamilies.join(",")} .*source_positions=true`),
          "production participant scene used the wrong resource families or positions");
          assert.equal(participantBoundsLogs.length, 1,
            "production participants did not emit projected mesh bounds");
          assert.match(participantBoundsLogs[0].text, /source_projection=true/,
            "production participant bounds omitted source-projected ships");
          const fighterParticipants = ready.participants
            .filter(({ kind, active }) => kind === "fighter-group" && active);
          const expectedFighterFamilies = [...new Map(fighterParticipants.map((participant) => [
            participant.active_close_resource,
            [participant.active_close_resource, participant.active_far_resource,
              participant.active_indicator_resource],
          ])).values()].sort((left, right) => left[0] - right[0]);
          assert.equal(fighterFamilyLogs.length, expectedFighterFamilies.length,
            "production participants loaded the wrong number of fighter families");
          assert.deepEqual(fighterFamilyLogs.map(({ text }) => {
            const match = text.match(/close=(\d+) far=(\d+) indicator=(\d+)/);
            assert.ok(match, "fighter family log omitted its exact resources");
            return match.slice(1).map(Number);
          }).sort((a, b) => a[0] - b[0]), expectedFighterFamilies);
          assert.ok(fighterFamilyLogs.every(({ text }) =>
            /dimensions=32x32,16x16,2x2 .*texture_filter=nearest alpha=opaque source=FUN_005c63f0/.test(text)),
          "fighter family load did not preserve dimensions or source texture state");
          // A field with no active fighter group (a failed trench run in a
          // Death Star battle, which brings no Imperial group) draws no
          // fighter scene.
          assert.equal(fighterSceneLogs.length,
            // The chase camera (F1, 0x005d9b10) sits close behind the lead
            // hull: the squadrons' sprites change detail (0x005d4af0) when
            // it starts, and the navigation journey's manual exit changes
            // them back.
            fighterParticipants.length === 0 ? 0 : scenario.fighter_detail_journey ? 9
              : scenario.navigation_camera_presentation ? 3
              : scenario.coruscant_chase ? 2 : 1,
            "production participants emitted the wrong fighter scene count");
          if (fighterParticipants.length > 0) {
            assert.match(fighterSceneLogs[0].text,
              new RegExp(`requested=${fighterParticipants.length} rendered=${fighterParticipants.length} `
                + ".*source_positions=true source=FUN_005ab650,FUN_005c63f0,0x005d4af0"),
              "production fighter scene used the wrong resource families, detail state, or positions");
            const initialFighterResources = parseFighterSceneResources(fighterSceneLogs[0]);
            assert.deepEqual(initialFighterResources.map(({ resource_id }) => resource_id),
              fighterParticipants.map(({ active_indicator_resource }) => active_indicator_resource));
          }
          if (scenario.navigation_camera_presentation) {
            assert.ok(cameraLogs.length >= 6,
              "navigation journey did not exercise the source tactical camera");
          } else if (scenario.group_presentation) {
            // The initial state and F12's follow: key 1 presses the
            // task-force button, which leaves the camera (FUN_0059b450).
            assert.equal(cameraLogs.length, 2,
              "group journey did not preserve its initial and follow-hotkey camera states");
          } else if (scenario.death_star_presentation) {
            // The free camera looks at its target's frame each frame
            // (0x005d9640, LookAt +0x90), and this journey's target is a
            // squadron on the moving trench run: one target, many states.
            const targets = new Set(cameraLogs.map(({ text }) =>
              text.match(/target_object_id=(\d+)/)?.[1]));
            assert.ok(cameraLogs.length >= 1 && targets.size === 1,
              "the Death Star journey's camera did not keep following its one target");
          } else {
            // The Umgul backdrop logs its first camera and 16 Rotate Left turns;
            // the Coruscant chase its first camera and F1's.
            assert.equal(cameraLogs.length,
              scenario.fighter_detail_journey ? 40 : scenario.umgul_backdrop ? 17
                : scenario.coruscant_chase ? 2 : 1,
              "production participants did not use the source tactical camera");
          }
          assert.equal(layoutLogs.length, 1,
            "production participants did not use the source tactical layout");
          // FUN_005d3770 picks each hull's model by its depth along the view
          // against 15 / field and 40 / field.
          const selectedResources = [...(participantSceneLogs[0].text
            .match(/resources=([^ ]+)/)?.[1] || "").matchAll(/:(\d+)/g)].map((match) => Number(match[1]));
          probes.push({
            type: "production-tactical-resource-families",
            families: participantFamilies,
            selected_resources: selectedResources,
            source_positions: true,
          });
          probes.push({
            type: "production-tactical-fighter-families",
            close_resources: fighterParticipants.map(({ active_close_resource }) =>
              active_close_resource),
            far_resources: fighterParticipants.map(({ active_far_resource }) =>
              active_far_resource),
            selected_resources: fighterParticipants.map(({ active_indicator_resource }) =>
              active_indicator_resource),
            source_thresholds: [5, 10],
            source_positions: true,
          });
          }
        } else {
          assert.equal(participantFamilyLogs.length, 0,
            "negative control loaded a production participant family");
          assert.equal(participantSceneLogs.length, 0,
            "negative control submitted production participants");
          assert.equal(participantBoundsLogs.length, 0,
            "negative control projected production participant bounds");
          assert.equal(fighterFamilyLogs.length, 0,
            "negative control loaded production fighter families");
          assert.equal(fighterSceneLogs.length, 0,
            "negative control submitted production fighter groups");
        }
      }
    }
    const runtimeMeasurements = battle ? await measureBrowserRuntime(page) : null;
    if (battle) {
      assert.ok(Number.isFinite(runtimeMeasurements.frame_interval_ms.p95)
        && runtimeMeasurements.frame_interval_ms.p95 < 100,
      "paused tactical render stalled during the 60-frame timing sample");
      assert.ok(Number.isSafeInteger(runtimeMeasurements.wasm_memory_bytes)
        && runtimeMeasurements.wasm_memory_bytes > 0,
      "WASM cold-load memory was not observable");
      probes.push({ type: "browser-runtime-measurements", ...runtimeMeasurements });
    }
    const comparison = compareScreenshot(id, stable.bytes, folder);
    const interactionComparisons = {};
    for (const capture of fs.readdirSync(folder)
      .filter((name) => name.endsWith(".png")
        && !["frame-a.png", "actual.png"].includes(name)
        && !name.startsWith("expected-") && !name.startsWith("diff-"))) {
      const name = capture.slice(0, -4);
      interactionComparisons[name] = compareScreenshot(
        id, fs.readFileSync(path.join(folder, capture)), folder, name,
      );
    }

    const expectedRequests = ["/", "/gl.js", "/open-rebellion-test.wasm", "/data/runtime.orpk"];
    // The trench run ends on its movie (ghidra/notes/movie-flow.md: 0x71
    // MDATA.201 on success, 0x70 MDATA.202 on failure), which the browser
    // plays from data/video.
    // The movie's proof: its request, or, when Escape ends it before the
    // browser asks for the file, the port's own "[cutscene] opened" line
    // for that stem.
    const trenchMovie = /trench-run-success/.test(id) ? "201" : /trench-run-failure/.test(id) ? "202" : null;
    const moviePath = trenchMovie && `/data/video/${trenchMovie}.webm`;
    const movieOpened = trenchMovie && consoleLines.some(({ text }) =>
      text.startsWith("[cutscene] opened ") && text.endsWith(`/${trenchMovie}.webm`));
    if (moviePath) {
      assert.ok(requests.some(({ url }) => url === moviePath) || movieOpened,
        `the trench run's movie ${trenchMovie} was neither requested nor opened`);
      expectedRequests.push(moviePath);
    }
    assert.deepEqual(requests.map(({ url: pathname }) => pathname).filter((url) => url !== moviePath).sort(),
      expectedRequests.filter((url) => url !== moviePath).sort());
    assert.ok(requests.every(({ status, aborted }) => aborted || status === 200), "startup has non-200 requests");
    assert.deepEqual(errors, [], `browser diagnostics for ${id}: ${errors.join("; ")}`);
    if (["mismatch", "mismatch_not_updated", "dimension_mismatch"].includes(comparison.status)) {
      fail(`pixel comparison failed for ${id}: ${comparison.different_pixels} pixels`);
    }
    for (const [capture, stateComparison] of Object.entries(interactionComparisons)) {
      if (["mismatch", "mismatch_not_updated", "dimension_mismatch"].includes(stateComparison.status)) {
        fail(`interaction pixel comparison failed for ${id}/${capture}: ${stateComparison.different_pixels} pixels`);
      }
    }

    result = {
      schema_version: schemaVersion,
      id,
      launch_attempts: launchAttempts,
      status: "pass",
      fixture_code: code,
      browser_version: browser.version(),
      browser_executable: executable,
      launch_arguments: browserManifest.launch_arguments,
      profile: "new-process-and-temporary-profile-per-scenario",
      muted: true,
      viewport,
      ready,
      screenshot_sha256: stable.second,
      two_frame_hashes: { first: stable.first, second: stable.second, equal: stable.first === stable.second },
      message_index_rail: messageIndexRail,
      first_frame: firstFrame,
      runtime_measurements: runtimeMeasurements,
      wasm_sha256: sha256(fs.readFileSync(path.join(site, "open-rebellion-test.wasm"))),
      runtime_pack_sha256: sha256(fs.readFileSync(path.join(site, "data/runtime.orpk"))),
      requests,
      errors,
      console: consoleLines,
      interaction_logs: consoleLines.filter(({ text }) => text.includes("[interface] command=")),
      probes,
      comparison,
      interaction_comparisons: interactionComparisons,
      cleanup: "pending",
    };
  } catch (error) {
    result = {
      schema_version: schemaVersion,
      id,
      launch_attempts: launchAttempts,
      status: "fail",
      error: String(error.stack || error),
      requests,
      errors,
      console: consoleLines,
      cleanup: "pending",
    };
  } finally {
    if (page) await page.close().catch((error) => errors.push(`page-close:${error}`));
    if (context) await context.close().catch((error) => errors.push(`context-close:${error}`));
    if (browser) await browser.close().catch((error) => errors.push(`browser-close:${error}`));
    result.cleanup = errors.some((error) => error.includes("-close:")) ? "failed" : "closed";
    if (result.cleanup === "failed") {
      result.status = "fail";
      result.error = `${result.error || ""}\nBrowser cleanup failed: ${errors.filter((error) => error.includes("-close:")).join("; ")}`.trim();
    }
    fs.writeFileSync(path.join(folder, "result.json"), `${JSON.stringify(result, null, 2)}\n`);
  }
  return result;
}

function isRetryableBrowserTimeout(result) {
  return result.status === "fail"
    && /(?:TimeoutError|Timeout \d+ms exceeded)/.test(result.error || "");
}

async function runScenario(server, executable, scenario, faction, viewport) {
  const executionAttempts = [];
  let result;
  for (let attempt = 1; attempt <= 2; attempt++) {
    result = await runScenarioOnce(server, executable, scenario, faction, viewport);
    const retryableTimeout = isRetryableBrowserTimeout(result);
    executionAttempts.push({
      attempt,
      status: result.status,
      cleanup: result.cleanup,
      retryable_timeout: retryableTimeout,
      launch_attempts: result.launch_attempts,
      ...(result.error ? { error: result.error } : {}),
    });
    if (result.status === "pass" || !retryableTimeout || attempt === 2) break;
    await new Promise((resolve) => setTimeout(resolve, 1_000));
  }

  result.execution_attempts = executionAttempts;
  const folder = path.resolve(runDir, result.id);
  fs.writeFileSync(path.join(folder, "result.json"), `${JSON.stringify(result, null, 2)}\n`);
  return result;
}

async function main() {
  fs.mkdirSync(runDir, { recursive: true });
  if (battle) {
    assert.equal(catalog.family, "TAC-01..TAC-07");
    assert.equal(catalog.fixture_namespace, 1);
    assert.deepEqual(catalog.scenarios.map(({ slug }) => slug),
      ["battle-entry", "battle-entry-proof-off", "lod-close", "lod-medium", "lod-far",
        "lod-journey", "camera-journey", "production-participants",
        "production-fighter-detail-journey",
        "production-group-presentation",
        "production-effect-presentation",
        "production-projectile-field-presentation",
        "production-selected-damage-presentation",
        "production-subsystem-field-command-presentation",
        "production-live-subsystem-damage-presentation",
        "production-subsystem-repair-mobility-presentation",
        "production-maneuver-movement-presentation",
        "production-command-assignment-presentation",
        "production-command-execution-presentation",
        "production-command-progression-presentation",
        "production-attack-targeting-presentation",
        "production-attack-target-lifecycle-presentation",
        "production-death-star-presentation",
        "production-battle-results-presentation",
        "production-battle-options-presentation",
        "production-battle-options-withdrawal",
        "production-death-star-laser-journey",
        "production-trench-run-success",
        "production-trench-run-failure",
        "production-selected-navigation-presentation",
        "production-navigation-camera-presentation",
        "production-empty-space-presentation",
        "production-detail-escort-presentation",
        "production-battle-alert-entry",
        "production-tactical-audio-routing",
        "production-hull-scale-a0",
        "production-participants-3d-off",
        "production-attack-target-lost-presentation",
        "production-battle-alert-retreat",
        "production-battle-withdrawal-report",
        "production-battle-last-ship-lost",
        "production-umgul-backdrop",
        "production-capital-panel-missions-tip",
        "production-coruscant-chase"]);
    assert.deepEqual(catalog.scenarios.map(({ tactical_proof }) => tactical_proof),
      [true, false, true, true, true, true, true, ...Array(37).fill(false)]);
    assert.deepEqual(catalog.scenarios.map(({ expected_lod_resource }) => expected_lod_resource),
      [2560, undefined, 2560, 2561, 2562, 2560, 2560, undefined, undefined,
        undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined,
        undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined,
        undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined,
        undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined,
        undefined, undefined, undefined]);
    assert.deepEqual(catalog.scenarios.map(({ lod_journey }) => Boolean(lod_journey)),
      Array.from({ length: 44 }, (_, index) => index === 5));
    assert.deepEqual(catalog.scenarios.map(({ camera_journey }) => Boolean(camera_journey)),
      Array.from({ length: 44 }, (_, index) => index === 6));
    assert.deepEqual(catalog.factions, ["alliance", "empire"]);
  } else {
    execFileSync(process.execPath, [path.join(here, "validate-catalog.mjs")], { stdio: "inherit" });
  }
  if (!noBuild) {
    const build = spawnSync("bash", [path.join(root, "scripts/build-interface-test-wasm.sh")], {
      cwd: root,
      stdio: "inherit",
      env: {
        ...process.env,
        PATH: `/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin:${path.join(os.homedir(), ".cargo/bin")}`,
      },
    });
    if (build.status !== 0) fail(`interface-test build failed: exit ${build.status}`);
  }
  const executable = browserExecutable();
  let server;
  const results = [];
  let selectedFactions = [];
  let selectedViewports = [];
  try {
    server = await startServer();
    const scenarios = scenarioFilter
      ? catalog.scenarios.filter(({ slug }) => slug === scenarioFilter)
      : battle
      ? catalog.scenarios
      : smoke
      ? catalog.scenarios.filter(({ slug }) => ["popular-support", "galaxy"].includes(slug))
      : catalog.scenarios;
    if (!scenarios.length) fail(`unknown ${battle ? "tactical" : "GID"} scenario: ${scenarioFilter}`);
    selectedFactions = smoke ? ["alliance", "empire"] : catalog.factions;
    selectedViewports = smoke ? [catalog.viewports[0]] : catalog.viewports;
    for (const scenario of scenarios) {
      for (const faction of selectedFactions) {
        for (const viewport of selectedViewports) {
          const result = await runScenario(server, executable, scenario, faction, viewport);
          results.push(result);
          process.stdout.write(`${result.status === "pass" ? "PASS" : "FAIL"} ${result.id}\n`);
        }
      }
    }
  } finally {
    if (server) await new Promise((resolve) => server.close(resolve));
  }

  if (battle && !scenarioFilter) {
    verifyTacticalProofNegativeControls(results, selectedFactions, selectedViewports);
    verifyTacticalLodFamily(results, selectedFactions, selectedViewports);
    verifyTacticalProductionParticipants(results, selectedFactions, selectedViewports);
  }

  writeContactSheet(results);
  const browserAttempts = summarizeBrowserAttempts(results);
  const summary = {
    schema_version: schemaVersion,
    family: catalog.family,
    status: results.every(({ status }) => status === "pass") ? "pass" : "fail",
    run_id: runId,
    cases: results.length,
    executions: browserAttempts.browser_executions,
    passed: results.filter(({ status }) => status === "pass").length,
    failed: results.filter(({ status }) => status === "fail").length,
    screenshots_with_matching_goldens: results.filter(({ comparison }) => comparison?.status === "match").length,
    unbaselined: results.filter(({ comparison }) => comparison?.status === "unbaselined").length,
    unstable_screenshots: results.filter(({ two_frame_hashes }) => two_frame_hashes && !two_frame_hashes.equal).length,
    four_request_startups: results.filter(({ requests }) => requests?.length === 4).length,
    ...browserAttempts,
    raw_artifacts: path.relative(root, runDir),
  };
  fs.writeFileSync(path.join(runDir, "summary.json"), `${JSON.stringify(summary, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
  if (summary.status !== "pass") process.exitCode = 1;
}

main().catch((error) => {
  process.stderr.write(`${error.stack || error}\n`);
  process.exitCode = 1;
});
