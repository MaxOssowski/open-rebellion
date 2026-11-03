#!/usr/bin/env node

// Build Selection gate: on each side, right-click a Manufacturing window band,
// choose Build, compare the window's static STRATEGY chrome exactly, Confirm,
// see the band take the product, then Stop it.

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
// `--a0=DIR` (.artifacts/a0-vm/build-selection) compares the fixed labels
// with A0 10-build (the Alliance's Build Selection at (192, 84)).
const a0Dir = process.argv.find((arg) => arg.startsWith("--a0="))?.slice(5);
const expectedRequests = ["/", "/data/runtime.orpk", "/gl.js", "/open-rebellion-test.wasm"];
// FUN_00437df0: a 210 by 261 window over background 10800.
const dimensions = { width: 210, height: 261 };
// Fixture code 55 is Scenario::BuildSelection (54) plus one.
const scenarioCode = 55;
// FUN_00438500: the side's title strip, 10801 for side 1, 10802 otherwise.
const factions = [
  { name: "alliance", byte: 1, title: 10801, galaxy: { x: 55, y: 40, width: 485, height: 350 }, stopPicture: 1038 },
  { name: "empire", byte: 2, title: 10802, galaxy: { x: 120, y: 40, width: 480, height: 355 }, stopPicture: 1039 },
];
// FUN_00438620 controls: (x, y, width, height, normal bitmap, native size).
const controls = [
  { x: 79, y: 90, width: 65, height: 18, id: 10606, keyed: true, native: [65, 18] },
  { x: 189, y: 196, width: 13, height: 8, id: 10610, keyed: true, native: [13, 8] },
  { x: 189, y: 205, width: 13, height: 8, id: 10612, keyed: true, native: [13, 8] },
  { x: 5, y: 224, width: 66, height: 33, id: 10592, keyed: true, native: [66, 33] },
  { x: 73, y: 224, width: 66, height: 33, id: 10594, keyed: true, native: [66, 33] },
  { x: 141, y: 224, width: 66, height: 33, id: 10596, keyed: true, native: [66, 33] },
  // port: the close box sits 17 pixels in from the right, as the mission
  // dialog's (hyp: the source chrome's placement is untraced).
  // A CoolStrobeButton, keyed on its first pixel (A0 menus2/11).
  { x: 193, y: 3, width: 14, height: 14, id: 10108, keyed: true, native: [14, 14] },
];
// Dynamic content, kept as captures: the title, the selected class, the
// labels, the four values and the number to build (FUN_00437f80,
// FUN_00438500).
const masks = [
  { x: 2, y: 2, width: 190, height: 15 },
  { x: 6, y: 22, width: 195, height: 63 },
  { x: 36, y: 110, width: 64, height: 23 },
  { x: 138, y: 110, width: 64, height: 23 },
  // The time labels' boxes (10..135) plus 5: the port's unhinted Arial
  // metrics run "Best Time To Completion:" to x 137 where GDI's hinted
  // glyphs end at 126 (A0 build-orders/15-t01, 2026-10-30).
  { x: 10, y: 145, width: 130, height: 15 },
  { x: 10, y: 165, width: 130, height: 15 },
  { x: 140, y: 145, width: 60, height: 15 },
  { x: 140, y: 165, width: 60, height: 15 },
  { x: 20, y: 196, width: 118, height: 15 },
  { x: 141, y: 196, width: 45, height: 17 },
];
const runId = `${new Date().toISOString().replace(/[:.]/g, "-")}-${process.pid}`;
const runDir = path.join(root, ".artifacts/interface-parity", `build-selection-${runId}`);

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


// The original's tooltip (win_tooltip.rs, ghidra/notes/tooltips.md): rest
// the pointer on a control for a second; a (255, 251, 240) tip with a
// (128, 128, 128) border, 19 high, shows at the pointer plus (0, 21), or
// above the pointer when it would pass y 440 (win_tooltip::TIP_BOTTOM).
async function tipShows(page, at) {
  await page.mouse.move(at.x, at.y, { steps: 4 });
  await page.waitForTimeout(1000);
  const png = PNG.sync.read(await page.screenshot());
  const pixel = (x, y) => {
    const i = (y * png.width + x) * 4;
    return [png.data[i], png.data[i + 1], png.data[i + 2]].join(",");
  };
  const top = at.y + 21 + 19 > 440 ? at.y - 19 : at.y + 21;
  const shown = pixel(at.x, top) === "128,128,128" && pixel(at.x, top + 18) === "128,128,128"
    && pixel(at.x + 2, top + 2) === "255,251,240";
  // The tip's box: the run of its grey bottom border (clear of the
  // pointer) through the pointer's column.
  let left = at.x;
  let right = at.x;
  while (left > 0 && pixel(left - 1, top + 18) === "128,128,128") left -= 1;
  while (right < png.width - 1 && pixel(right + 1, top + 18) === "128,128,128") right += 1;
  await page.mouse.move(2, 2, { steps: 4 });
  await page.waitForTimeout(100);
  return shown && { left: left - at.x, top: top - at.y, right: right - at.x };
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
  // FUN_005fd170: the first byte of the bits is the bitmap's key.
  const key = bytes[paletteOffset + bytes[dataOffset] * 4 + 2] << 16
    | bytes[paletteOffset + bytes[dataOffset] * 4 + 1] << 8 | bytes[paletteOffset + bytes[dataOffset] * 4];
  png.key = key;
  return png;
}

function blitClipped(destination, source, x, y, width = source.width, height = source.height, keyed = false) {
  // FUN_005fc140 blits at the bitmap's own size; the control clips the rest.
  // A CoolStrobeButton blits keyed (FUN_00602d30 -> FUN_005fd0f0): its key
  // colour shows the window behind.
  for (let row = 0; row < Math.min(height, source.height); row += 1) {
    if (y + row >= destination.height) break;
    const columns = Math.min(width, source.width, destination.width - x);
    for (let column = 0; column < columns; column += 1) {
      const from = (row * source.width + column) * 4;
      const colour = source.data[from] << 16 | source.data[from + 1] << 8 | source.data[from + 2];
      if (keyed && colour === source.key) continue;
      destination.data.set(source.data.subarray(from, from + 4), ((y + row) * destination.width + x + column) * 4);
    }
  }
}

async function frames(page) {
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}


function resource(source, id, width, height) {
  return decodeIndexedBmp(fs.readFileSync(path.join(source, `${id}.bmp`)), width, height);
}

function composeExpected(source, faction) {
  const expected = resource(source, 10800, dimensions.width, dimensions.height);
  blitClipped(expected, resource(source, faction.title, 240, 17), 2, 2);
  for (const control of controls) {
    blitClipped(expected, resource(source, control.id, ...control.native), control.x, control.y,
      control.width, control.height, control.keyed ?? false);
  }
  return expected;
}

function masked(x, y) {
  return masks.some((mask) => x >= mask.x && x < mask.x + mask.width
    && y >= mask.y && y < mask.y + mask.height);
}

function crop(screenshotBytes, origin) {
  const screenshot = PNG.sync.read(screenshotBytes);
  const cropped = new PNG({ width: dimensions.width, height: dimensions.height });
  for (let y = 0; y < dimensions.height; y += 1) {
    const start = ((origin.y + y) * screenshot.width + origin.x) * 4;
    cropped.data.set(screenshot.data.subarray(start, start + dimensions.width * 4), y * dimensions.width * 4);
  }
  return cropped;
}

function compare(actual, expected, label, directory) {
  let differentPixels = 0;
  let checked = 0;
  const diff = new PNG({ width: expected.width, height: expected.height });
  for (let y = 0; y < expected.height; y += 1) {
    for (let x = 0; x < expected.width; x += 1) {
      const offset = (y * expected.width + x) * 4;
      if (masked(x, y)) {
        diff.data.set([0, 0, 96, 255], offset);
        continue;
      }
      checked += 1;
      const matches = [0, 1, 2].every((channel) => actual.data[offset + channel] === expected.data[offset + channel]);
      if (!matches) differentPixels += 1;
      diff.data.set(matches ? [0, 0, 0, 255] : [255, 0, 80, 255], offset);
    }
  }
  fs.writeFileSync(path.join(directory, `${label}-actual.png`), PNG.sync.write(actual));
  fs.writeFileSync(path.join(directory, `${label}-expected.png`), PNG.sync.write(expected));
  fs.writeFileSync(path.join(directory, `${label}-diff.png`), PNG.sync.write(diff));
  return { label, pixels_checked: checked, different_pixels: differentPixels };
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

async function production(page) {
  return page.evaluate(() => window.__openRebellionInterfaceProduction);
}

// Right-click the band, then click the menu row the report names.
async function bandOrder(page, band, rowName, directory, label) {
  await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
  await click(page, { x: band.left + band.width / 2, y: band.top + band.height / 2 }, "right");
  await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
    null, { timeout: 10_000 });
  const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
  fs.writeFileSync(path.join(directory, `${label}-menu.png`), await page.screenshot({ animations: "disabled" }));
  const index = menu[rowName];
  assert.notEqual(index, null, `the band's menu lists ${rowName}`);
  const row = (menu.height - 2) / menu.rows;
  await click(page, { x: menu.left + menu.width / 2, y: menu.top + 2 + row * (index + 0.5) });
  await page.mouse.move(2, 2);
  await frames(page);
  return menu;
}

// The fixed labels (FUN_00438500: the time labels in font 10, "Number to
// build" in font 4, DrawTextA through theme::paint_gdi_text) against A0
// 10-build: each ink box within a pixel, relative to the window. A0 draws
// black and white; the port's antialiased edges count from 120.
function labelsAgainstA0(port, origin) {
  const a0 = PNG.sync.read(fs.readFileSync(path.join(a0Dir, "10-build.png")));
  const ink = (png, ox, oy, [x0, y0, x1, y1], threshold) => {
    let box = null;
    for (let y = y0; y < y1; y += 1) {
      for (let x = x0; x < x1; x += 1) {
        const i = ((oy + y) * png.width + ox + x) * 4;
        if (Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) <= threshold) continue;
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
      }
    }
    return box;
  };
  const checks = {};
  for (const [name, region] of [["best-completion", [8, 144, 135, 161]], ["best-deployment", [8, 164, 135, 181]],
    ["number-to-build", [20, 198, 139, 210]]]) {
    const a = ink(a0, 192, 84, region, 200);
    const p = ink(port, origin.x, origin.y, region, 120);
    checks[name] = { a0: a, port: p };
    assert.ok(a && p && a.every((value, k) => Math.abs(value - p[k]) <= 1), `${name}: ${JSON.stringify(checks[name])}`);
  }
  return checks;
}

// The idle overview against A0 build-selection2/11 (the Empire's
// Coruscant, every band idle, the window at (203, 91); the fixture's at
// (225, 76)): each band's status line (TEXTSTRA 6198..6200 at the band's
// (5, 16), FUN_00458080) and the troops' and facilities' "1:1" counts
// (FUN_00457690). Left, top and bottom within 2 pixels; the right edge
// within 3, Liberation Sans's footed "1" against GDI Arial's (the
// accepted text deviation). A0's Coruscant has no shipyard ("0:0"), so
// the ships' count is not compared.
function idleBandsAgainstA0(port) {
  const file = path.join(root, ".artifacts/a0-vm/build-selection2/11-stopped.png");
  if (!fs.existsSync(file)) return null;
  const a0 = PNG.sync.read(fs.readFileSync(file));
  const ink = (png, ox, oy, [x0, y0, x1, y1], threshold) => {
    let box = null;
    for (let y = y0; y < y1; y += 1) {
      for (let x = x0; x < x1; x += 1) {
        const k = ((oy + y) * png.width + ox + x) * 4;
        if (Math.min(png.data[k], png.data[k + 1], png.data[k + 2]) <= threshold) continue;
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
      }
    }
    return box;
  };
  const checks = {};
  for (const [name, region] of [["ships-status", [58, 72, 215, 85]], ["troops-status", [58, 153, 215, 166]],
    ["facilities-status", [58, 234, 215, 247]], ["troops-count", [6, 200, 52, 216]], ["facilities-count", [6, 280, 52, 296]]]) {
    const a = ink(a0, 203, 91, region, 200);
    const p = ink(port, 225, 76, region, 120);
    checks[name] = { a0: a, port: p };
    assert.ok(a && p && [0, 1, 3].every((k) => Math.abs(a[k] - p[k]) <= 2) && Math.abs(a[2] - p[2]) <= 3,
      `${name}: ${JSON.stringify(checks[name])}`);
  }
  return checks;
}

async function quantityAndCancelCase(page, origin, band, directory, faction) {
  const read = async () => {
    await page.mouse.move(2, 2);
    await frames(page);
    await page.waitForTimeout(150);
    await frames(page);
    return production(page);
  };
  const numbers = (report) => report.build_selection_values.map((text) => Number.parseInt(text, 10));
  const one = await read();
  assert.equal(one.build_selection_quantity, 1, JSON.stringify(one));
  const unit = numbers(one);
  assert.ok(unit.every(Number.isFinite), JSON.stringify(one.build_selection_values));
  const up = { x: origin.x + 189 + 6, y: origin.y + 196 + 4 };
  const down = { x: origin.x + 189 + 6, y: origin.y + 205 + 4 };
  await click(page, up);
  await read();
  await click(page, up);
  const three = await read();
  fs.writeFileSync(path.join(directory, "quantity-three.png"), await page.screenshot({ animations: "disabled" }));
  assert.equal(three.build_selection_quantity, 3, JSON.stringify(three));
  const tripled = numbers(three);
  // Costs and the single yard's completion scale with the number; the
  // deployment does not (A0 03: 18, 9, "72 Days", "0 Days").
  assert.deepEqual(tripled, [unit[0] * 3, unit[1] * 3, unit[2] * 3, unit[3]], JSON.stringify([unit, tripled]));
  await click(page, down);
  const two = await read();
  assert.equal(two.build_selection_quantity, 2, JSON.stringify(two));
  assert.deepEqual(numbers(two), [unit[0] * 2, unit[1] * 2, unit[2] * 2, unit[3]]);
  await click(page, { x: origin.x + 141 + 33, y: origin.y + 224 + 16 });
  const cancelled = await read();
  fs.writeFileSync(path.join(directory, "cancelled-screen.png"), await page.screenshot({ animations: "disabled" }));
  assert.equal(cancelled.build_selection_open, false, "Cancel closes Build Selection");
  assert.equal(cancelled.bands.find(({ area }) => area === "troops").queued, 0, "Cancel builds nothing");
  await bandOrder(page, band, "build_row", directory, "rebuild");
  await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.build_selection_open === true,
    null, { timeout: 10_000 });
  const reopened = await read();
  assert.equal(reopened.build_selection_quantity, 1, "Build reopens the window at 1");
  // The Empire's Imperial Army Regiment against A0 build-selection2/01
  // (Coruscant, one training facility): 6, 3, "24 Days", "0 Days"
  // (FUN_00538220, FUN_00528d30). The class list (button at (79, 90))
  // shows two 70-pixel items under the box at (6, 85).
  let a0Class = null;
  if (faction.name === "empire") {
    // The open list starts at the shown class, so its second item is the
    // next class; step until the regiment shows.
    const seen = [];
    for (let step = 0; step < 12 && (await read()).build_selection_class !== "Imperial Army Regiment"; step += 1) {
      seen.push((await read()).build_selection_class);
      await click(page, { x: origin.x + 79 + 32, y: origin.y + 90 + 9 });
      await read();
      await click(page, { x: origin.x + 6 + 97, y: origin.y + 85 + 35 + 70 });
    }
    const chosen = await read();
    assert.equal(chosen.build_selection_class, "Imperial Army Regiment", JSON.stringify([seen, chosen]));
    assert.deepEqual(chosen.build_selection_values, ["6", "3", "24 Days", "0 Days"], JSON.stringify(chosen));
    a0Class = { class: chosen.build_selection_class, values: chosen.build_selection_values };
  }
  return { unit: one.build_selection_values, three: three.build_selection_values, two: two.build_selection_values,
    a0_class: a0Class };
}

// A0 build-selection2/09 (Windows 10 VM, the Empire's Stop on Coruscant's
// troops band, day 144): the confirmation at (148, 52).
const A0_STOP = { file: "09-stopped.png", x: 148, y: 52 };

async function stopConfirmationCase(page, source, faction, directory) {
  const { galaxy } = faction;
  // FUN_00606980 centres truncating (mission-dialog.mjs).
  const origin = {
    x: Math.floor(galaxy.x + (galaxy.width - 424) / 2),
    y: Math.floor(galaxy.y + (galaxy.height - 331) / 2),
  };
  await page.waitForTimeout(300);
  await frames(page);
  const bytes = await page.screenshot({ animations: "disabled" });
  fs.writeFileSync(path.join(directory, "stop-confirmation-screen.png"), bytes);
  const shot = PNG.sync.read(bytes);
  const result = { origin, picture: faction.stopPicture };
  // The picture, 400 by 200, every pixel.
  const bitmap = resource(source, faction.stopPicture, 400, 200);
  let differing = 0;
  for (let y = 0; y < 200; y += 1) {
    for (let x = 0; x < 400; x += 1) {
      const from = (y * 400 + x) * 4;
      const at = ((origin.y + 30 + y) * shot.width + origin.x + 12 + x) * 4;
      if (shot.data[at] !== bitmap.data[from] || shot.data[at + 1] !== bitmap.data[from + 1]
        || shot.data[at + 2] !== bitmap.data[from + 2]) differing += 1;
    }
  }
  result.picture_differing = differing;
  assert.equal(differing, 0, `stop picture ${faction.stopPicture} differs in ${differing} pixels`);
  // The template's two lines (0x7053, font 4, 16 apart): each ink box's
  // left, top and bottom within 2 pixels of A0's, relative to the window,
  // and its width within 15% (the accepted text-width deviation: GDI's
  // hinted Arial against Liberation Sans; A0 ends "of the" at 307, the
  // port at 334).
  const a0File = path.join(root, ".artifacts/a0-vm/build-selection2", A0_STOP.file);
  if (faction.name === "empire" && fs.existsSync(a0File)) {
    const a0 = PNG.sync.read(fs.readFileSync(a0File));
    const ink = (png, ox, oy, [x0, y0, x1, y1], threshold) => {
      let box = null;
      for (let y = y0; y < y1; y += 1) {
        for (let x = x0; x < x1; x += 1) {
          const k = ((oy + y) * png.width + ox + x) * 4;
          if (Math.min(png.data[k], png.data[k + 1], png.data[k + 2]) <= threshold) continue;
          box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
        }
      }
      return box;
    };
    result.lines = {};
    for (const [name, region] of [["first", [20, 242, 346, 258]], ["second", [20, 258, 346, 274]]]) {
      const a = ink(a0, A0_STOP.x, A0_STOP.y, region, 200);
      const p = ink(shot, origin.x, origin.y, region, 120);
      result.lines[name] = { a0: a, port: p };
      const ratio = a && p ? (p[2] - p[0] + 1) / (a[2] - a[0] + 1) : 0;
      assert.ok(a && p && [0, 1, 3].every((k) => Math.abs(a[k] - p[k]) <= 2) && ratio > 0.85 && ratio < 1.15,
        `stop ${name} line: ${JSON.stringify(result.lines[name])}`);
    }
  }
  await click(page, { x: origin.x + 355 + 25, y: origin.y + 244 + 17 });
  await page.mouse.move(2, 2);
  await frames(page);
  return result;
}

// Fixture code 89 is Scenario::BuildSelectionDestination (88) plus one.
async function destinationCase(page, faction, directory) {
  const fixtureCode = 89 | (faction.byte << 8);
  await page.goto(`${page.url().split("?")[0]}?fixture-code=${fixtureCode}`, { waitUntil: "load", timeout: 30_000 });
  await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status, null, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.bands?.length === 3,
    null, { timeout: 10_000 });
  const before = await production(page);
  const home = before.bands.map(({ destination }) => destination);
  assert.ok(home.every((name) => name === home[0] && name.length > 0), JSON.stringify(home));
  const troops = before.bands.find(({ area }) => area === "troops");
  const menu = await bandOrder(page, troops, "destination_row", directory, "destination");
  assert.ok(menu.target_screen_x > 0 && menu.target_name && menu.target_name !== home[0], JSON.stringify(menu));
  await click(page, { x: menu.target_screen_x, y: menu.target_screen_y });
  await page.mouse.move(2, 2);
  await page.waitForFunction((name) => window.__openRebellionInterfaceProduction?.bands
    ?.find(({ area }) => area === "troops")?.destination === name, menu.target_name, { timeout: 10_000 });
  fs.writeFileSync(path.join(directory, "destination-set.png"), await page.screenshot({ animations: "disabled" }));
  const after = await production(page);
  const shown = Object.fromEntries(after.bands.map(({ area, destination }) => [area, destination]));
  assert.deepEqual(shown, { ships: home[0], troops: menu.target_name, facilities: home[0] }, JSON.stringify(shown));
  // Released on the Manufacturing window itself (its yard column), the
  // Destination is the window's system again (A0 build-selection2/27:
  // Chandrila back to Coruscant).
  const moved = after.bands.find(({ area }) => area === "troops");
  await bandOrder(page, moved, "destination_row", directory, "destination-again");
  await click(page, { x: 360 + 20, y: 76 + 150 });
  await page.mouse.move(2, 2);
  await page.waitForFunction((name) => window.__openRebellionInterfaceProduction?.bands
    ?.find(({ area }) => area === "troops")?.destination === name, home[0], { timeout: 10_000 });
  fs.writeFileSync(path.join(directory, "destination-window.png"), await page.screenshot({ animations: "disabled" }));
  return { home: home[0], target: menu.target_name, shown, back_on_window: home[0] };
}

// Fixture code 95 is Scenario::BuildSelectionDefensesDestination (94) plus
// one: the secondary system's Defenses window at the galaxy view's left.
// A band's Destination released on it is that system (A0
// build-selection2/25: Coruscant's Defenses window).
async function defensesDestinationCase(page, faction, directory) {
  const fixtureCode = 95 | (faction.byte << 8);
  await page.goto(`${page.url().split("?")[0]}?fixture-code=${fixtureCode}`, { waitUntil: "load", timeout: 30_000 });
  await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status, null, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.bands?.length === 3,
    null, { timeout: 10_000 });
  const before = await production(page);
  const home = before.bands.find(({ area }) => area === "ships").destination;
  const ships = before.bands.find(({ area }) => area === "ships");
  const menu = await bandOrder(page, ships, "destination_row", directory, "defenses-destination");
  assert.ok(menu.target_name && menu.target_name !== home, JSON.stringify(menu));
  // The Defenses window's personnel list, at its (100, 150).
  await click(page, { x: faction.galaxy.x + 5 + 100, y: faction.galaxy.y + 5 + 150 });
  await page.mouse.move(2, 2);
  await page.waitForFunction((name) => window.__openRebellionInterfaceProduction?.bands
    ?.find(({ area }) => area === "ships")?.destination === name, menu.target_name, { timeout: 10_000 });
  fs.writeFileSync(path.join(directory, "defenses-destination.png"), await page.screenshot({ animations: "disabled" }));
  return { home, target: menu.target_name };
}

// Fixture code 96 is Scenario::BuildSelectionFleetDestination (95) plus
// one: the player's fleet at the secondary system, its Fleet window at the
// galaxy view's left. A band's Destination released on the fleet is
// refused: a production manager's must be a system (FUN_00537180, 1/0x25),
// as A0 build-selection2/30, 32, 35 kept the band's.
async function fleetDestinationCase(page, faction, directory, log) {
  const fixtureCode = 96 | (faction.byte << 8);
  await page.goto(`${page.url().split("?")[0]}?fixture-code=${fixtureCode}`, { waitUntil: "load", timeout: 30_000 });
  await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status, null, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.bands?.length === 3,
    null, { timeout: 10_000 });
  const before = await production(page);
  const ships = before.bands.find(({ area }) => area === "ships");
  const home = ships.destination;
  await bandOrder(page, ships, "destination_row", directory, "fleet-destination");
  // The Fleet window's first entry, in its left list at (4..95, 29..).
  await click(page, { x: faction.galaxy.x + 5 + 40, y: faction.galaxy.y + 5 + 42 });
  await page.mouse.move(2, 2);
  await log("the fleet release is refused",
    /command=0x214 destination=production_destination status=refused reason=0x25$/);
  await frames(page, 4);
  const after = (await production(page)).bands.find(({ area }) => area === "ships").destination;
  fs.writeFileSync(path.join(directory, "fleet-destination.png"), await page.screenshot({ animations: "disabled" }));
  assert.equal(after, home, "the refused release changed the band's Destination");
  return { home, after };
}

// Fixture code 88 is Scenario::BuildSelectionOverMaintenance (87) plus one.
const overMaintenanceCode = 88;
const A0_OVER_MAINTENANCE = { file: "11-build-selection.png", x: 255, y: 87 };
// `--a0-manufacturing=DIR`, default .artifacts/a0-vm/manufacturing.
function a0ManufacturingDir() {
  const dir = process.argv.find((arg) => arg.startsWith("--a0-manufacturing="))?.slice(19)
    ?? path.join(root, ".artifacts/a0-vm/manufacturing");
  return fs.existsSync(path.join(dir, A0_OVER_MAINTENANCE.file)) ? dir : null;
}

async function overMaintenanceCase(page, faction, directory) {
  const fixtureCode = overMaintenanceCode | (faction.byte << 8);
  await page.goto(`${page.url().split("?")[0]}?fixture-code=${fixtureCode}`, { waitUntil: "load", timeout: 30_000 });
  await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status, null, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.bands?.length === 3,
    null, { timeout: 10_000 });
  const troops = (await production(page)).bands.find(({ area }) => area === "troops");
  await bandOrder(page, troops, "build_row", directory, "over-maintenance-build");
  await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.build_selection_open === true,
    null, { timeout: 10_000 });
  const opened = await production(page);
  const origin = { x: Math.round(opened.build_selection_left), y: Math.round(opened.build_selection_top) };
  await stableWindow(page, origin, directory, "over-maintenance");
  const port = PNG.sync.read(fs.readFileSync(path.join(directory, "over-maintenance-screen.png")));
  const result = { origin };
  const a0Manufacturing = a0ManufacturingDir();
  result.a0 = a0Manufacturing;
  if (a0Manufacturing) {
    const a0 = PNG.sync.read(fs.readFileSync(path.join(a0Manufacturing, A0_OVER_MAINTENANCE.file)));
    // Confirm, 66 by 33 at (73, 224): every pixel, the keyed corner
    // showing the window's background in both.
    let differing = 0;
    for (let y = 0; y < 33; y += 1) {
      for (let x = 0; x < 66; x += 1) {
        const i = ((origin.y + 224 + y) * port.width + origin.x + 73 + x) * 4;
        const j = ((A0_OVER_MAINTENANCE.y + 224 + y) * a0.width + A0_OVER_MAINTENANCE.x + 73 + x) * 4;
        if (port.data[i] !== a0.data[j] || port.data[i + 1] !== a0.data[j + 1] || port.data[i + 2] !== a0.data[j + 2]) {
          differing += 1;
        }
      }
    }
    result.confirm_differing = differing;
    assert.equal(differing, 0, `disabled Confirm differs from A0 in ${differing} pixels`);
    // The four "n/a" (14358): each ink box within 2 pixels of A0's,
    // relative to the window (GDI hinting is the accepted deviation).
    const ink = (png, ox, oy, [x0, y0, w, h], threshold) => {
      let box = null;
      for (let y = y0; y < y0 + h; y += 1) {
        for (let x = x0; x < x0 + w; x += 1) {
          const k = ((oy + y) * png.width + ox + x) * 4;
          if (Math.min(png.data[k], png.data[k + 1], png.data[k + 2]) <= threshold) continue;
          box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
        }
      }
      return box;
    };
    result.values = {};
    for (const [name, field] of [["refined", [36, 110, 64, 23]], ["maintenance", [138, 110, 64, 23]],
      ["completion", [140, 145, 60, 15]], ["deployment", [140, 165, 60, 15]]]) {
      const a = ink(a0, A0_OVER_MAINTENANCE.x, A0_OVER_MAINTENANCE.y, field, 200);
      const p = ink(port, origin.x, origin.y, field, 120);
      result.values[name] = { a0: a, port: p };
      assert.ok(a && p && a.every((value, k) => Math.abs(value - p[k]) <= 2), `${name}: ${JSON.stringify(result.values[name])}`);
    }
  }
  // A click on the disabled Confirm builds nothing and keeps the window.
  await click(page, { x: origin.x + 73 + 33, y: origin.y + 224 + 16 });
  await page.mouse.move(2, 2);
  await frames(page);
  await page.waitForTimeout(300);
  const after = await production(page);
  assert.equal(after.build_selection_open, true, "the disabled Confirm closes nothing");
  assert.equal(after.bands.find(({ area }) => area === "troops").queued, 0, "the disabled Confirm builds nothing");
  return result;
}

async function stableWindow(page, origin, directory, label) {
  await page.waitForTimeout(150);
  await frames(page);
  const first = await page.screenshot({ animations: "disabled" });
  await frames(page);
  const second = await page.screenshot({ animations: "disabled" });
  fs.writeFileSync(path.join(directory, `${label}-screen.png`), second);
  const a = crop(first, origin);
  const b = crop(second, origin);
  assert.equal(sha256(PNG.sync.write(a)), sha256(PNG.sync.write(b)), `window did not stabilize (${label})`);
  return b;
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
    await page.evaluate(() => document.fonts.ready);
    await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.bands?.length === 3,
      null, { timeout: 10_000 });
    const before = await production(page);
    const troops = before.bands.find(({ area }) => area === "troops");
    assert.equal(troops.queued, 0, "the training facility starts idle");
    const overviewBytes = await page.screenshot({ animations: "disabled" });
    fs.writeFileSync(path.join(directory, "overview-screen.png"), overviewBytes);
    const idleChecks = faction.name === "empire" ? idleBandsAgainstA0(PNG.sync.read(overviewBytes)) : null;

    // Build (0x212) opens Build Selection (FUN_0041d640).
    const buildMenu = await bandOrder(page, troops, "build_row", directory, "build");
    await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.build_selection_open === true,
      null, { timeout: 10_000 });
    const opened = await production(page);
    const origin = { x: Math.round(opened.build_selection_left), y: Math.round(opened.build_selection_top) };
    const captured = await stableWindow(page, origin, directory, "opened");
    const comparisons = [compare(captured, composeExpected(source, faction), "opened", directory)];
    // The class list (FUN_0060bed0, two 70-pixel items): its button at
    // (79, 90) opens it under the box; captured, then closed again by the
    // same button (A0 manufacturing/24).
    await click(page, { x: origin.x + 79 + 32, y: origin.y + 90 + 9 });
    await page.mouse.move(2, 2);
    await frames(page);
    await page.waitForTimeout(300);
    await frames(page);
    fs.writeFileSync(path.join(directory, "class-list.png"), await page.screenshot({ animations: "disabled" }));
    await click(page, { x: origin.x + 79 + 32, y: origin.y + 90 + 9 });
    await page.mouse.move(2, 2);
    await frames(page);
    const textChecks = a0Dir && faction.name === "alliance"
      ? labelsAgainstA0(PNG.sync.read(fs.readFileSync(path.join(directory, "opened-screen.png"))), origin) : null;
    // FUN_00437f80 / FUN_00438620's tips (TEXTSTRA 0x190c, 0x1906..0x190b,
    // and 0x1903 on the close box through FUN_00606e10): every control
    // shows one.
    const tips = {};
    for (const [name, [x, y, w, h]] of [["quantity", [141, 196, 45, 17]], ["list", [79, 90, 65, 18]],
      ["more", [189, 196, 13, 8]], ["fewer", [189, 205, 13, 8]], ["encyclopedia", [5, 224, 66, 33]],
      ["done", [73, 224, 66, 33]], ["cancel", [141, 224, 66, 33]], ["close", [193, 3, 14, 14]]]) {
      tips[name] = await tipShows(page, { x: origin.x + x + Math.floor(w / 2), y: origin.y + y + Math.floor(h / 2) });
    }
    assert.ok(Object.values(tips).every(Boolean), JSON.stringify(tips));
    // A0 build-selection/tip-* (Windows 10 VM, Alliance, the same points in
    // the window): each box (left, top, right inclusive) from the pointer.
    // Left and top must match; the width may differ by the accepted text
    // deviation. A0's Done was disabled there and showed no tip.
    const A0_TIPS = { quantity: [0, 21, 72], list: [0, 21, 78], more: [0, 21, 48], fewer: [0, 21, 52],
      encyclopedia: [0, 21, 75], cancel: [0, 21, 41], close: [0, 21, 34] };
    for (const [name, [left, top, right]] of Object.entries(A0_TIPS)) {
      const box = tips[name];
      assert.equal(box.top, top, `${name} tip top ${box.top} vs A0 ${top}`);
      assert.equal(box.left, left, `${name} tip left ${box.left} vs A0 ${left}`);
      const ratio = (box.right - box.left + 1) / (right - left + 1);
      assert.ok(ratio > 0.85 && ratio < 1.15, `${name} tip width ratio ${ratio.toFixed(3)}`);
    }

    // The number's arrows and Cancel, as A0 build-selection2/01..06 (the
    // Empire on Coruscant, one training facility): the up arrow at
    // (189, 196) steps the number and the values follow it (1: 6, 3,
    // "24 Days"; 3: 18, 9, "72 Days"), the down arrow at (189, 205) steps
    // it back (2: 12, 6, "48 Days"); Cancel at (141, 224) closes the
    // window with the band unchanged, and Build reopens it at 1
    // (FUN_00438c30, FUN_00438c60, FUN_00438800).
    const stepping = await quantityAndCancelCase(page, origin, troops, directory, faction);

    // Confirm (0x65, FUN_00438980) builds one unit of the selected class.
    await click(page, { x: origin.x + 73 + 33, y: origin.y + 224 + 16 });
    await page.mouse.move(2, 2);
    await page.waitForFunction(() => {
      const report = window.__openRebellionInterfaceProduction;
      const band = report?.bands?.find(({ area }) => area === "troops");
      return report && !report.build_selection_open && band?.queued > 0;
    }, null, { timeout: 10_000 });
    const built = (await production(page)).bands.find(({ area }) => area === "troops");
    assert.ok(built.product.length > 0, "the band names its product");
    await page.waitForTimeout(200);
    await frames(page);
    const confirmedBytes = await page.screenshot({ animations: "disabled" });
    fs.writeFileSync(path.join(directory, "confirmed-screen.png"), confirmedBytes);
    // Its menu closed, the band is no longer selected: its title strip's
    // text-free right end (window (195..215, 141..150)) is the unselected
    // strip's, as A0 build-selection2/07 shows it for the Empire (the window
    // at (203, 91)).
    const stripA0 = path.join(root, ".artifacts/a0-vm/build-selection2/07-confirmed.png");
    let stripDiffering = null;
    if (faction.name === "empire" && fs.existsSync(stripA0)) {
      const a0 = PNG.sync.read(fs.readFileSync(stripA0));
      const port = PNG.sync.read(confirmedBytes);
      stripDiffering = 0;
      for (let y = 141; y < 151; y += 1) {
        for (let x = 195; x < 216; x += 1) {
          const i = ((76 + y) * port.width + 225 + x) * 4;
          const j = ((91 + y) * a0.width + 203 + x) * 4;
          if (port.data[i] !== a0.data[j] || port.data[i + 1] !== a0.data[j + 1] || port.data[i + 2] !== a0.data[j + 2]) {
            stripDiffering += 1;
          }
        }
      }
      assert.equal(stripDiffering, 0, `the confirmed band's strip differs from A0 07 in ${stripDiffering} pixels`);
    }

    // Stop (0x213) asks first (FUN_00487cc0, FUN_0049a350 case 0x213):
    // the 424 by 331 confirmation centred in the galaxy view, its picture
    // 0x40e/0x40f at (12, 30); the checkmark clears the band (manual p. 84).
    const stopMenu = await bandOrder(page, built, "stop_row", directory, "stop");
    const stopConfirmation = await stopConfirmationCase(page, source, faction, directory);
    await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.bands
      ?.find(({ area }) => area === "troops")?.queued === 0, null, { timeout: 10_000 });
    fs.writeFileSync(path.join(directory, "stopped-screen.png"), await page.screenshot({ animations: "disabled" }));

    // Reserved (0x216) on the band: enabled, unchecked; chosen, the
    // manager is Reserved and the row shows the check mark 11902 in the
    // menu's icon column (A0 menus3/32, 33).
    const reserveMenu = await bandOrder(page, built, "reserved_row", directory, "reserve");
    assert.deepEqual(reserveMenu.row_states.at(-1), ["Reserved", true], JSON.stringify(reserveMenu));
    assert.equal(reserveMenu.reserved_checked, false, JSON.stringify(reserveMenu));
    await page.waitForTimeout(400);
    await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
    await click(page, { x: built.left + built.width / 2, y: built.top + built.height / 2 }, "right");
    await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
      null, { timeout: 10_000 });
    const reservedMenu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
    fs.writeFileSync(path.join(directory, "reserved-menu.png"), await page.screenshot({ animations: "disabled" }));
    assert.equal(reservedMenu.reserved_checked, true, JSON.stringify(reservedMenu));
    // The check mark widens the menu by its 17-pixel column but leaves the
    // rows their height (FUN_004abf60, FUN_004abbf0): A0 menus3/32, 33 are
    // 90 and 107 wide, 122 tall both. The fixture reports screen pixels.
    const scale = reserveMenu.height / 122;
    assert.equal(Math.round((reservedMenu.width - reserveMenu.width) / scale), 17,
      JSON.stringify([reserveMenu.width, reservedMenu.width, scale]));
    assert.equal(reservedMenu.height, reserveMenu.height, JSON.stringify([reserveMenu.height, reservedMenu.height]));
    await page.keyboard.press("Escape");
    await page.mouse.move(2, 2);
    await frames(page);

    // A manufacturing facility's menu: its page cell's right click lists
    // FUN_0051d990's Encyclopedia and Status and the facility's one Scrap
    // (FUN_005399e0), all enabled on the player's own yard as in A0
    // status/28-fac-menu.png.
    const [tx, ty, tw, th] = (await production(page)).training_tab;
    await click(page, { x: tx + tw / 2, y: ty + th / 2 });
    await page.mouse.move(2, 2);
    await frames(page);
    await page.waitForTimeout(400);
    // Each yard page shows its first facility's 67 by 35 picture keyed at
    // the window's (9, 79) (FUN_004568a0: the list at (8, 77), the
    // picture centred in a 69 by 40 cell). A0 manufacturing/02 and 03
    // (Coruscant) put 9014 and 9022 there pixel for pixel. The fixture's
    // System window stands at (225, 76); the tabs are 32 apart.
    const pages = [];
    for (const [offset, picture, label] of [[-32, 9006, "shipyards"], [32, 9022, "construction-yards"], [0, 9014, "training"]]) {
      await click(page, { x: tx + tw / 2 + offset, y: ty + th / 2 });
      await page.mouse.move(2, 2);
      await frames(page);
      await page.waitForTimeout(300);
      await frames(page);
      const bytes = await page.screenshot({ animations: "disabled" });
      fs.writeFileSync(path.join(directory, `${label}-page.png`), bytes);
      const shot = PNG.sync.read(bytes);
      const bitmap = resource(source, picture, 67, 35);
      let checked = 0;
      let differing = 0;
      for (let y = 0; y < 35; y += 1) {
        for (let x = 0; x < 67; x += 1) {
          const from = (y * 67 + x) * 4;
          const colour = bitmap.data[from] << 16 | bitmap.data[from + 1] << 8 | bitmap.data[from + 2];
          if (colour === 0x0000ff) continue;
          const at = ((76 + 79 + y) * shot.width + 225 + 9 + x) * 4;
          checked += 1;
          if (shot.data[at] !== bitmap.data[from] || shot.data[at + 1] !== bitmap.data[from + 1]
            || shot.data[at + 2] !== bitmap.data[from + 2]) differing += 1;
        }
      }
      pages.push({ label, picture, checked, differing });
    }
    assert.ok(pages.every(({ checked, differing }) => checked > 400 && differing === 0), JSON.stringify(pages));
    fs.writeFileSync(path.join(directory, "training-page.png"), await page.screenshot({ animations: "disabled" }));
    const [cx, cy, cw, ch] = (await production(page)).first_cell;
    await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
    await click(page, { x: cx + cw / 2, y: cy + ch / 2 }, "right");
    await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
      null, { timeout: 10_000 });
    const facilityMenu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
    fs.writeFileSync(path.join(directory, "facility-menu.png"), await page.screenshot({ animations: "disabled" }));
    assert.deepEqual(facilityMenu.row_states, [["Encyclopedia", true], ["Status", true], ["Scrap", true]],
      JSON.stringify(facilityMenu));

    assert.ok(comparisons.every(({ different_pixels: d }) => d === 0),
      JSON.stringify(comparisons.filter(({ different_pixels: d }) => d !== 0)));
    assert.deepEqual(requests.map(({ url }) => url).sort(), [...expectedRequests].sort());
    assert.ok(requests.every(({ status }) => status === 200), "startup has non-200 requests");
    assert.deepEqual(errors, [], `browser diagnostics: ${errors.join("; ")}`);

    // The side over its maintenance (fixture 88: no mine or refinery):
    // FUN_0052e970 refuses the order, so FUN_00439160 writes "n/a" in all
    // four values and Confirm shows its disabled art 11620 (FUN_00438620)
    // and does nothing. A0 manufacturing/11 (the Empire at -402, window at
    // (255, 87)) is the reference.
    const overMaintenance = await overMaintenanceCase(page, faction, directory);
    // A band's Destination (0x214) released on another system's planet
    // sets that band's area alone (A0 build-selection2/21: the ships band's
    // Destination on Chandrila; the others keep Coruscant).
    const destination = await destinationCase(page, faction, directory);
    const defensesDestination = await defensesDestinationCase(page, faction, directory);
    const logged = async (description, pattern) => {
      const deadline = Date.now() + 10_000;
      while (Date.now() < deadline) {
        const line = consoleLines.find(({ text }) => pattern.test(text));
        if (line) return line.text;
        await new Promise((resolve) => setTimeout(resolve, 50));
      }
      throw new Error(`${description}: ${pattern} not logged`);
    };
    const fleetDestination = await fleetDestinationCase(page, faction, directory, logged);
    assert.deepEqual(errors, [], `browser diagnostics: ${errors.join("; ")}`);
    result = {
      status: "pass",
      faction: faction.name,
      fixture_code: fixtureCode,
      origin,
      build_menu: buildMenu,
      stop_menu: stopMenu,
      reserve_menu: reserveMenu,
      pages,
      reserved_menu: reservedMenu,
      facility_menu: facilityMenu,
      over_maintenance: overMaintenance,
      stepping,
      destination,
      defensesDestination,
      fleetDestination,
      idle_checks: idleChecks,
      strip_differing_after_confirm: stripDiffering,
      stop_confirmation: stopConfirmation,
      built: { product: built.product, queued: built.queued },
      comparisons,
      textChecks,
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
  const ids = [10800, 10108, ...new Set([...controls.map(({ id }) => id), ...factions.map(({ title }) => title)])]
    .filter((id, index, all) => all.indexOf(id) === index).sort((a, b) => a - b);
  const aggregate = createHash("sha256");
  for (const id of ids) {
    aggregate.update(`${id}\0`);
    aggregate.update(fs.readFileSync(path.join(source, `${id}.bmp`)));
  }
  const summary = {
    schema_version: 1,
    family: "build-selection",
    scope: "test-only Build Selection window (FUN_00437df0) reached from a Manufacturing window band's Build: static STRATEGY chrome compared exactly; title, class item, labels, values and number masked and kept as captures; Confirm builds and Stop clears the band",
    status: passed ? "pass" : "fail",
    browser_version: browserManifest.version,
    browser_executable: executable,
    launch_arguments: browserManifest.launch_arguments,
    viewport: { width: 640, height: 480, device_scale_factor: 1 },
    window: dimensions,
    masks,
    source: { dll: "STRATEGY.DLL", resource_ids: ids, aggregate_sha256: aggregate.digest("hex") },
    factions: results,
    wasm_sha256: sha256(fs.readFileSync(path.join(site, "open-rebellion-test.wasm"))),
    runtime_pack_sha256: sha256(fs.readFileSync(path.join(site, "data/runtime.orpk"))),
  };
  fs.writeFileSync(path.join(runDir, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
  if (!passed) throw new Error(JSON.stringify(results.filter((result) => result.status !== "pass"), null, 2));
  process.stdout.write(`${JSON.stringify({ run_dir: runDir, status: summary.status, factions: results.map(({ faction, status, comparisons, built }) => ({ faction, status, comparisons, built })) }, null, 2)}\n`);
}

await main();
