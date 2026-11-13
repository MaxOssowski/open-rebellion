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
// `--a0=DIR` (.artifacts/a0-vm/system-windows) compares the System window's
// text with A0 07-icon-dbl (Yavin's window at (315, 87), the Alliance).
const a0Dir = process.argv.find((arg) => arg.startsWith("--a0="))?.slice(5);
const expectedRequests = ["/", "/data/runtime.orpk", "/gl.js", "/open-rebellion-test.wasm"];
// Fixture codes are the Scenario index plus one (interface_test_fixture.rs).
const QUADRANTS = 50;
// CoruscantSeized: Coruscant the Alliance's, as A0 coruscant-676/14; the
// setup's second system is Coruscant. A case with `scenario` runs it.
const CORUSCANT_SEIZED = 107;
// CoruscantSquadrons: the Empire's Coruscant with ten TIE Fighter
// squadrons, as A0 2026-11-11-icon-menu-topic/05; the second system again.
const CORUSCANT_SQUADRONS = 109;
// CoruscantRegiments: as CoruscantSeized with the Alliance's two Army
// Regiments there, seen by the Alliance (A0 2026-11-13-drag/30..33).
const CORUSCANT_REGIMENTS = 115;
// QuadrantsNeutralFleet: as QUADRANTS with the player's fleet at rest at
// the neutral system (one ship that bombards) and one of the player's
// squadrons on the primary planet (setup.neutral_fleets).
const QUADRANTS_NEUTRAL_FLEET = 117;
// QuadrantsFreeTeam: as QUADRANTS with the player's agent at the primary
// free (no mission), so the defenses icon's team may move.
const QUADRANTS_FREE_TEAM = 118;
// A0 captures these cases read, by path under .artifacts/a0-vm, with the
// SHA-256 of each (its directory's SHA256SUMS).
const A0_FILES = {
  "2026-11-13-icon-drag-move/20-fleet-icon-drag-start.png":
    "511d44431ecd9424c9df400a14a192e6364b4acfc411304982ffb9b4860baf53",
  "2026-11-13-icon-drag-move/21-fleet-icon-dragged-over-kubindi.png":
    "dc359652bd9afb198573dfdf92d1d06ac06d2d0a7e9f554199e6485c10a6ffd3",
  "2026-11-13-icon-drag-move/22-fleet-icon-released-on-kubindi.png":
    "b717628ba2c1b9709dcc5ab26c1457ed3d6d9ff7ff84ba3573bbf0c9bdef4fac",
  "2026-11-13-hyperspace-icons/00-kubindi-fleet-icon-menu-day321.png":
    "775692778ec3581b1e892d608916dda0ccc4c85c21b0f5c84016f9ebf5112c94",
  "2026-11-13-defenses-missions-drop/00-jerijador-defenses-icon-menu.png":
    "f65ba930f3ab886eb69966376cb74467056c11aeb1cdd9678674b388d429980e",
  "2026-11-13-defenses-missions-drop/03-defenses-icon-mid-drag.png":
    "669f219f71b7801b5b5d46f092fdfd71617954261ba8b0e09753fc409202f023",
  "2026-11-13-defenses-missions-drop/05-released-on-yavin.png":
    "c2a7263457120023412405a72c675176ec1ed4b2ac8d03504689faefd4c7d286",
  "2026-11-13-defenses-missions-drop/21-jerijador-defenses-icon-menu-after.png":
    "839a46138ecf8ba750a0811567fb5febf4582f021ec56f0fccfc1c984bc6a372",
  "2026-11-13-defenses-missions-drop/25-missions-icon-mid-drag.png":
    "e088f02e83502bb0bcfdfd857641121c0384311a2b67c109537e9047e878684d",
  "2026-11-13-defenses-missions-drop/27-released-on-jerijador.png":
    "e90b648691f86c26ea12affd677ea1601d1619cbb50846ac2b0fd092e416d40a",
  "2026-11-13-defenses-missions-drop/28-3s-after-drop.png":
    "6c4af518ed4feb63bb91856340d2d52006fd043bdb919d0ce96caef55bd82d54",
};
const factions = [
  { name: "alliance", byte: 1, side: 1, other: 2, galaxyRight: 540 },
  { name: "empire", byte: 2, side: 2, other: 1, galaxyRight: 600 },
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

function gokresDirectory() {
  const candidates = [
    process.env.REBELLION_GOKRES_BMP_DIR,
    path.join(root, "data/base/ui/gokres-dll/BMP"),
  ].filter(Boolean);
  const directory = candidates.find((candidate) => fs.existsSync(path.join(candidate, "17472.bmp")));
  if (!directory) {
    throw new Error("owned GOKRES.DLL BMP extraction is unavailable; set REBELLION_GOKRES_BMP_DIR");
  }
  return directory;
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


// The original's tooltip (win_tooltip.rs, A0 a0-vm/tooltip/153..157): rest
// the pointer on a control for a second; a (255, 251, 240) tip with a
// (128, 128, 128) border, 19 high, shows at the pointer plus (0, 21).
async function tipShows(page, at) {
  await page.mouse.move(at.x, at.y, { steps: 4 });
  await page.waitForTimeout(1000);
  const png = PNG.sync.read(await page.screenshot());
  const pixel = (x, y) => {
    const i = (y * png.width + x) * 4;
    return [png.data[i], png.data[i + 1], png.data[i + 2]].join(",");
  };
  const top = at.y + 21;
  const shown = pixel(at.x, top) === "128,128,128" && pixel(at.x, top + 18) === "128,128,128"
    && pixel(at.x + 2, top + 2) === "255,251,240";
  await page.mouse.move(2, 2, { steps: 4 });
  await page.waitForTimeout(100);
  return shown;
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
const usedGokres = new Set();

function resource(source, id) {
  used.add(id);
  return decodeIndexedBmp(fs.readFileSync(path.join(source, `${id}.bmp`)));
}

function gokres(id) {
  usedGokres.add(id);
  return decodeIndexedBmp(fs.readFileSync(path.join(gokresDirectory(), `${id}.bmp`)));
}

// A window-sized bitmap's region (x, y, w, h) against the screenshot at the
// window's screen corner, every pixel opaque.
function compareRegion(screenshot, corner, image, region, label, directory) {
  const [x0, y0, width, height] = region;
  const crop = new PNG({ width, height });
  for (let y = 0; y < height; y += 1) {
    const from = ((y0 + y) * image.width + x0) * 4;
    crop.data.set(image.data.subarray(from, from + width * 4), y * width * 4);
  }
  return compareIcon(screenshot, [corner[0] + x0, corner[1] + y0], crop, label, directory, false);
}

// The palette-blue matte every quadrant icon carries (bmp_cache.rs,
// uses_blue_screen_transparency, 10771..=10790).
function blueKey(data, offset) {
  return data[offset] < 32 && data[offset + 1] < 32 && data[offset + 2] > 192;
}

// The icon's opaque pixels against the screenshot at the overlay's top-left
// (port: paint_native draws the bitmap at its own size there). The matte
// shows whatever lies behind, so it stays out of the check.
function compareIcon(screenshot, rect, icon, label, directory, keyed = true) {
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
      if (typeof keyed === "function" ? keyed(x, y) : keyed && blueKey(icon.data, from)) {
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

// An A0 capture of A0_FILES, its hash checked, or null when it is absent.
function a0Capture(relative) {
  const file = path.join(root, ".artifacts/a0-vm", relative);
  if (!fs.existsSync(file)) return null;
  const bytes = fs.readFileSync(file);
  assert.equal(sha256(bytes), A0_FILES[relative], `${relative} is not the recorded capture`);
  return PNG.sync.read(bytes);
}

// Where the opaque pixels of `icon` (blue key out) match `png` best with
// its corner in [x0, x1] by [y0, y1], skipping `skip(x, y)` screen pixels
// (a captured cursor): the corner and the pixels that differ there.
function templateMatch(png, icon, [x0, x1], [y0, y1], skip = () => false) {
  let best = null;
  for (let y = y0; y <= y1; y += 1) {
    for (let x = x0; x <= x1; x += 1) {
      let checked = 0;
      let different = 0;
      for (let iy = 0; iy < icon.height; iy += 1) {
        for (let ix = 0; ix < icon.width; ix += 1) {
          const from = (iy * icon.width + ix) * 4;
          if (blueKey(icon.data, from) || skip(x + ix, y + iy)) continue;
          checked += 1;
          const at = ((y + iy) * png.width + x + ix) * 4;
          if ([0, 1, 2].some((k) => png.data[at + k] !== icon.data[from + k])) different += 1;
        }
      }
      if (!best || different < best.different) best = { x, y, checked, different };
    }
  }
  return best;
}

// An object menu's frame and rows against A0's, relative to each corner:
// the two-pixel ring's frame-art pixels (grey 128, 192 or 255) and each
// 20-pixel row's ink box and the class of its commonest ink colour. Ink
// is the text's colours only: a near-grey above 100 (white, grey and the
// port's antialiased edges) or the side's highlight red or green (the
// submenu arrow too); the shaded fill
// stays at or below 92, but a bright planet under it can pass 100 in A0.
// `cursor` (x, y, w, h from the corner) is the arrow A0's capture shows
// there, left out of both. The fill shades another scene and is not
// compared.
function textInk([r, g, b]) {
  return (Math.max(r, g, b) - Math.min(r, g, b) <= 16 && Math.max(r, g, b) > 100) || (r === 255 && g === 0 && b === 0) || (r === 0 && g === 255 && b === 0);
}

function menuAgainstA0(port, [mx, my], a0, [ax, ay], { width, height }, cursor) {
  const pixel = (png, x, y) => Array.from(png.data.subarray((y * png.width + x) * 4, (y * png.width + x) * 4 + 3));
  const masked = (x, y) => x >= cursor[0] && y >= cursor[1] && x < cursor[0] + cursor[2] && y < cursor[1] + cursor[3];
  const rows = (height - 2) / 20;
  const rowInk = (png, x0, y0, row) => {
    let box = null;
    const colours = new Map();
    for (let y = 2 + 20 * row; y < Math.min(22 + 20 * row, height - 2); y += 1) {
      for (let x = 3; x < width - 3; x += 1) {
        if (masked(x, y)) continue;
        const value = pixel(png, x0 + x, y0 + y);
        if (!textInk(value)) continue;
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
        colours.set(value.join(","), (colours.get(value.join(",")) ?? 0) + 1);
      }
    }
    const colour = [...colours.entries()].sort((a, b) => b[1] - a[1])[0]?.[0];
    // The class by the row's brightest grey: the port's glyphs are
    // antialiased, so a grey row's commonest ink can sit a few steps under
    // 128 where A0's is all 128 (the accepted text deviation).
    const brightest = Math.max(-1, ...[...colours.keys()].map((key) => key.split(",").map(Number))
      .filter(([r, g, b]) => r === g && g === b).map(([r]) => r));
    const kind = colour === "0,255,0" || colour === "255,0,0" ? "highlight"
      : brightest >= 250 ? "white"
        : Math.abs(brightest - 128) <= 1 ? "grey" : colour;
    return { box, colour, kind };
  };
  const portRows = [...Array(rows).keys()].map((row) => rowInk(port, mx, my, row));
  const a0Rows = [...Array(rows).keys()].map((row) => rowInk(a0, ax, ay, row));
  const inkDiffs = a0Rows.map(({ box }, row) => (box && portRows[row].box
    ? box.map((value, k) => portRows[row].box[k] - value) : null));
  const frameArt = (value) => value[0] === value[1] && value[1] === value[2] && [128, 192, 255].includes(value[0]);
  let ring = 0;
  const differing = [];
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      if ((x > 1 && x < width - 2 && y > 1 && y < height - 2) || masked(x, y)) continue;
      ring += 1;
      const [a, b] = [pixel(a0, ax + x, ay + y), pixel(port, mx + x, my + y)];
      if (frameArt(a) !== frameArt(b) || (frameArt(a) && a.join() !== b.join())) differing.push([x, y, a, b]);
    }
  }
  return { port: portRows, a0: a0Rows, inkDiffs, ring, differing: differing.length, first: differing.slice(0, 8) };
}

async function click(page, point) {
  await page.mouse.move(point.x, point.y);
  await frames(page);
  await page.mouse.down();
  await frames(page);
  await page.mouse.up();
  await frames(page);
}

async function rightClick(page, point) {
  await page.mouse.move(point.x, point.y);
  await frames(page);
  await page.mouse.down({ button: "right" });
  await frames(page);
  await page.mouse.up({ button: "right" });
  await frames(page);
}

async function frames(page, count = 1) {
  for (let index = 0; index < count; index += 1) {
    await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  }
}

// Two clicks inside egui's double-click time (0.3 s); a slow software
// frame between them would let it pass.
async function doubleClick(page, point) {
  await page.mouse.move(point.x, point.y);
  await frames(page);
  await page.mouse.dblclick(point.x, point.y);
  await frames(page, 2);
}

// The System window's fixed lines and counts (FUN_00458080, FUN_00457690)
// against A0 07-icon-dbl: each ink box within a pixel of A0's, relative to
// the window. The port's window stands one pixel right of and below its
// reported origin, as the tab strip's art shows. DrawTextA sets these
// lines (theme::paint_gdi_text); the glyphs are Liberation's.
async function systemWindowText(page, origin, directory) {
  await page.mouse.move(2, 470);
  await page.waitForTimeout(800);
  const portBytes = await page.screenshot();
  fs.writeFileSync(path.join(directory, "system-window-whole.png"), portBytes);
  const port = PNG.sync.read(portBytes);
  const a0 = PNG.sync.read(fs.readFileSync(path.join(a0Dir, "07-icon-dbl.png")));
  const white = (png, x, y) => {
    const i = (y * png.width + x) * 4;
    return Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) > 200;
  };
  const ink = (png, [ox, oy], [x0, y0, x1, y1]) => {
    let box = null;
    for (let y = y0; y < y1; y += 1) {
      for (let x = x0; x < x1; x += 1) {
        if (!white(png, ox + x, oy + y)) continue;
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
      }
    }
    return box;
  };
  const checks = {};
  for (const [name, region] of [["no-ships", [57, 72, 200, 88]], ["no-troops", [57, 153, 200, 169]],
    ["no-facilities", [57, 234, 200, 250]], ["count-troops", [8, 197, 52, 213]],
    ["count-facilities", [8, 278, 52, 294]]]) {
    const a = ink(a0, [315, 87], region);
    const p = ink(port, [origin[0] + 1, origin[1] + 1], region);
    checks[name] = { a0: a, port: p };
    assert.ok(a && p && a.every((value, k) => Math.abs(value - p[k]) <= 1), `${name}: ${JSON.stringify(checks[name])}`);
  }
  // The whole window, 226 by 304 (FUN_0045aac0), against A0's at (314, 86):
  // the port's window art stands at its reported origin. Every pixel must
  // match but the text objects' boxes, whose glyphs are Liberation's and
  // whose title and destinations name another system (Yavin in A0): the
  // title (FUN_00458080, from (19, 2)) and, per band 81 rows apart, its
  // title, status and destination lines and its count (FUN_00457690).
  const masks = [[19, 2, 175, 14]];
  for (let band = 0; band < 3; band += 1) {
    const y = band * 81;
    masks.push([58, 59 + y, 160, 13], [58, 74 + y, 160, 14], [58, 115 + y, 160, 13], [18, 120 + y, 22, 14]);
  }
  const masked = (x, y) => masks.some(([mx, my, mw, mh]) => x >= mx && y >= my && x < mx + mw && y < my + mh);
  const rgbAt = (png, x, y) => {
    const i = (y * png.width + x) * 4;
    return (png.data[i] << 16) | (png.data[i + 1] << 8) | png.data[i + 2];
  };
  let compared = 0;
  let differing = 0;
  for (let y = 0; y < 304; y += 1) {
    for (let x = 0; x < 226; x += 1) {
      if (masked(x, y)) continue;
      compared += 1;
      if (rgbAt(port, origin[0] + x, origin[1] + y) !== rgbAt(a0, 314 + x, 86 + y)) differing += 1;
    }
  }
  checks.whole = { compared, differing, masks: masks.length };
  assert.ok(compared > 45_000 && differing === 0, `the whole System window: ${JSON.stringify(checks.whole)}`);
  return checks;
}

// A 235 by 304 window (FUN_004ac120) on `port` at `portOrigin` against A0's
// `a0File` at `a0Origin`, every pixel but `masks` ([x, y, w, h] in window
// pixels): the text objects, whose glyphs are Liberation's, and what each
// game's own state draws. Returns the count and the first differing pixels.
function wholeWindow(port, portOrigin, a0File, a0Origin, masks) {
  const a0 = PNG.sync.read(fs.readFileSync(a0File));
  const masked = (x, y) => masks.some(([mx, my, mw, mh]) => x >= mx && y >= my && x < mx + mw && y < my + mh);
  const rgbAt = (png, x, y) => {
    const i = (y * png.width + x) * 4;
    return (png.data[i] << 16) | (png.data[i + 1] << 8) | png.data[i + 2];
  };
  let compared = 0;
  const differing = [];
  for (let y = 0; y < 304; y += 1) {
    for (let x = 0; x < 235; x += 1) {
      if (masked(x, y)) continue;
      compared += 1;
      if (rgbAt(port, portOrigin[0] + x, portOrigin[1] + y) !== rgbAt(a0, a0Origin[0] + x, a0Origin[1] + y)) {
        differing.push([x, y]);
      }
    }
  }
  return { a0: path.basename(a0File), compared, differing: differing.length, first: differing.slice(0, 12),
    masks: masks.length };
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


// The System Defenses window (type 10, defenses_window.rs) the bottom-left
// icon opens: FUN_004a8790's tab art (normal, selected = +1, empty = +2;
// one gray id for side 0/3 on the side-art tabs), and the pages the
// fixture stocks.
const TAB_BASE = {
  batteries: [10550, 10550, 10550],
  shields: [10553, 10553, 10553],
  squadrons: [10562, 10556, 10559],
  regiments: [10569, 10563, 10566],
  personnel: [10576, 10570, 10573],
};
function tabArt(name, side, selected, empty) {
  const base = TAB_BASE[name][side === 1 || side === 2 ? side : 0];
  if ((side !== 1 && side !== 2) && !["batteries", "shields"].includes(name)) return base;
  return base + (selected ? 1 : empty ? 2 : 0);
}
const PAGES = ["personnel", "regiments", "squadrons", "shields", "batteries"];
// troop_mini: the Alliance Fleet Regiment and Stormtrooper Regiment minis.
const REGIMENT_MINI = { 1: 17472, 2: 17536 };

function defensesWindow(observation, dat) {
  return observation.defenses_windows.find((window) => window.system_dat_id === dat);
}

// The window's screen corner from its personnel tab at (28, 20).
function corner(window) {
  const [left, top] = window.tabs.find(([name]) => name === "personnel")[1];
  return [left - 28, top - 20];
}

function compareTabs(screenshot, source, window, directory, prefix) {
  const checks = [];
  for (const [name, rect] of window.tabs) {
    const index = PAGES.indexOf(name);
    const id = tabArt(name, window.side, window.page === name, window.counts[index] === 0);
    // The tabs key their first DIB pixel's colour (bmp_cache.rs; A0
    // garrison-clean shows their pure blue drawn), so only that colour
    // stays out of the check.
    const icon = resource(source, id);
    const corner = (icon.height - 1) * icon.width * 4;
    const keyed = (x, y) => {
      const i = (y * icon.width + x) * 4;
      return [0, 1, 2].every((k) => icon.data[i + k] === icon.data[corner + k]);
    };
    checks.push({ id, ...compareIcon(screenshot, rect, icon, `${prefix}-tab-${name}`, directory, keyed) });
  }
  return checks;
}

function assertExact(checks) {
  for (const check of checks) {
    assert.ok(check.pixels_checked > 0, `${check.label} checked nothing`);
    assert.equal(check.different_pixels, 0,
      `${check.label} (${check.id}): ${check.different_pixels} of ${check.pixels_checked} pixels differ`);
  }
}

async function openDefenses(page, setup, system) {
  const [left, top] = quadrant(setup, system, "defenses");
  const at = { x: Math.round(left) + 4, y: Math.round(top) + 9 };
  await doubleClick(page, at);
  const dat = system === "primary" ? setup.primary_dat_id : setup.second_dat_id;
  const opened = await until(page, `the ${system} defenses icon opens the Defenses window`,
    (o, value) => o.defenses_windows.some((window) => window.system_dat_id === value), dat);
  return { at, dat, opened, window: defensesWindow(opened, dat) };
}

// The Missions window (type 11, missions_window.rs) the bottom-right icon
// opens: a mission's GOKRES mini is 0x4000 + (TEXTSTRA id & 0xfff), 0x5000 +
// for side 2 (FUN_004a1590; Diplomacy 0x2c10); the row frame by the player's
// side; the tabs by the selected mission's side, pressed while selected
// (FUN_004a0ca0); the rail icon by FUN_004a1f60's side (FUN_004a21c0).
const DIPLOMACY_MINI = { 1: 0x4c10, 2: 0x5c10 };
const MISSION_FRAME = { 1: 11127, 2: 11128 };
const MISSION_TABS = { 1: { agents: 11560, decoys: 11562 }, 2: { agents: 11565, decoys: 11567 } };
const MISSIONS_RAIL = { 1: 11539, 2: 11540 };

function missionsWindow(observation, dat) {
  return observation.missions_windows.find((window) => window.system_dat_id === dat);
}

// A mission row's mini below its name: the list draws the name over the
// picture from (1, 0) (FUN_006083c0's DrawTextA after FUN_00609960), so the
// first rows hold text in the original as in the port: one line of font 10,
// a 14-row cell (FUN_0049f540: the list's slot +0x18 with 10).
const ROW_TEXT_BAND = 14;
function compareRowMini(screenshot, rect, image, label, directory) {
  const height = image.height - ROW_TEXT_BAND;
  const crop = new PNG({ width: image.width, height });
  image.data.copy(crop.data, 0, ROW_TEXT_BAND * image.width * 4);
  return compareIcon(screenshot, [rect[0], rect[1] + ROW_TEXT_BAND], crop, label, directory);
}

function compareMissionTabs(screenshot, source, window, directory, prefix) {
  return window.tabs.map(([name, rect]) => {
    const id = MISSION_TABS[window.tab_side][name] + (window.tab === name ? 1 : 0);
    return { id, ...compareIcon(screenshot, rect, resource(source, id), `${prefix}-tab-${name}`, directory) };
  });
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
      // FUN_0045bbb0: the Alliance headquarters' mark (0x388) over its planet,
      // which the Empire does not see here, and a side's uprising mark
      // (0x389 side 1, 0x38a side 2) keyed into the player's revolting
      // system's planet.
      assert.ok(setup.headquarters && setup.uprising, JSON.stringify(setup));
      const headquarters = { id: 904, ...compareIcon(screenshot, setup.headquarters, resource(source, 904), "headquarters", directory) };
      const uprisingId = faction.side === 1 ? 905 : 906;
      const uprising = { id: uprisingId, ...compareIcon(screenshot, setup.uprising, resource(source, uprisingId), "uprising", directory) };
      assert.equal(uprising.different_pixels, 0, `uprising mark: ${uprising.different_pixels} of ${uprising.pixels_checked} differ`);
      if (faction.side === 1) {
        assert.equal(headquarters.different_pixels, 0, `headquarters mark: ${headquarters.different_pixels} of ${headquarters.pixels_checked} differ`);
      } else {
        assert.ok(headquarters.different_pixels > headquarters.pixels_checked / 2, "the Empire sees no headquarters mark");
      }
      // An explored neutral system (A0 sector-b, the Sesswenna sector):
      // its name white (FUN_0045bbb0 leaves the item's white for side 0)
      // and its system icon the neutral art 10787, as A0 shows by
      // Chandrila, Yaga Minor and five more, every pixel.
      assert.ok(setup.neutral, JSON.stringify(setup));
      const [planetRect] = setup.neutral;
      // The icon's place beside the planet: the first spot near it where
      // every pixel of 10787 shows (A0 puts it at the planet's left).
      const icon = resource(source, 10787);
      let neutral = null;
      for (let dy = -30; dy <= 40 && !neutral; dy++) {
        for (let dx = -40; dx <= 40 && !neutral; dx++) {
          const at = [planetRect[0] + dx, planetRect[1] + dy];
          let differing = 0;
          let checked = 0;
          for (let y = 0; y < icon.height && differing === 0; y++) {
            for (let x = 0; x < icon.width; x++) {
              const from = (y * icon.width + x) * 4;
              if (blueKey(icon.data, from)) continue;
              const to = ((Math.round(at[1]) + y) * screenshot.width + Math.round(at[0]) + x) * 4;
              checked++;
              if ([0, 1, 2].some((c) => icon.data[from + c] !== screenshot.data[to + c])) { differing++; break; }
            }
          }
          if (differing === 0 && checked > 0) neutral = { id: 10787, at, pixels_checked: checked, different_pixels: 0 };
        }
      }
      assert.ok(neutral, "the neutral system shows the neutral icon 10787 beside its planet");
      const png = screenshot;
      let white = 0;
      let sided = 0;
      // The name stands under the tracks, from 17 rows below the planet.
      const nameTop = Math.round(planetRect[1] + planetRect[3] + 17 * setup.scale);
      for (let y = nameTop; y < nameTop + 12 * setup.scale; y++) {
        for (let x = Math.round(planetRect[0] - 20 * setup.scale); x < Math.round(planetRect[0] + planetRect[2] + 20 * setup.scale); x++) {
          const i = (y * png.width + x) * 4;
          const [r, g, b] = png.data.subarray(i, i + 3);
          if (r > 200 && g > 200 && b > 200) white++;
          if ((r > 200 && g < 60 && b < 60) || (g > 200 && r < 60 && b < 60)) sided++;
        }
      }
      assert.ok(white > 10 && sided === 0, `the neutral name: ${white} white, ${sided} side-coloured pixels`);
      // A destroyed system (A0 menus3/05, Averam after the Death Star):
      // the asteroid field 10240 in place of its planet.
      assert.ok(setup.destroyed, JSON.stringify(setup));
      // Keyed on its black first pixel (bmp_cache), so those show the window.
      const field = resource(source, 10240);
      const key = field.data.subarray(0, 3);
      let fieldChecked = 0;
      let fieldDiffering = 0;
      for (let y = 0; y < field.height; y++) {
        for (let x = 0; x < field.width; x++) {
          const from = (y * field.width + x) * 4;
          if ([0, 1, 2].every((c) => field.data[from + c] === key[c])) continue;
          const at = ((Math.round(setup.destroyed[1]) + y) * screenshot.width
            + Math.round(setup.destroyed[0]) + x) * 4;
          fieldChecked++;
          if ([0, 1, 2].some((c) => field.data[from + c] !== screenshot.data[at + c])) fieldDiffering++;
        }
      }
      const destroyed = { id: 10240, pixels_checked: fieldChecked, different_pixels: fieldDiffering };
      assert.equal(fieldDiffering, 0, `asteroid field: ${fieldDiffering} of ${fieldChecked} differ`);
      // A press selects it, and it draws its pair's second bitmap 10788
      // (FUN_0045ca80 state 0, FUN_004593e0 WM_PAINT), as A0
      // tooltip/151-menu shows on Chandrila, every pixel.
      await click(page, { x: Math.round(neutral.at[0]) + 4, y: Math.round(neutral.at[1]) + 9 });
      const selectedScreen = await stableScreen(page, directory, "neutral-selected");
      const neutralSelected = { id: 10788, ...compareIcon(selectedScreen, [neutral.at[0], neutral.at[1], 28, 19],
        resource(source, 10788), "neutral-selected", directory) };
      assert.equal(neutralSelected.different_pixels, 0,
        `the selected neutral icon: ${neutralSelected.different_pixels} of ${neutralSelected.pixels_checked} differ`);
      neutral.selected = neutralSelected;
      // Its defenses icon (the player's battery there) takes the system's
      // side too: the neutral pair 10789 at rest, 10790 selected
      // (FUN_0045ce80, FUN_0045ca80 state 0); A0 shows both, every pixel
      // (sector-b's neutral planets; modal/07-after).
      assert.ok(setup.neutral_defenses, JSON.stringify(setup));
      const defensesRest = { id: 10789, ...compareIcon(selectedScreen, setup.neutral_defenses,
        resource(source, 10789), "neutral-defenses", directory) };
      await click(page, { x: Math.round(setup.neutral_defenses[0]) + 4, y: Math.round(setup.neutral_defenses[1]) + 9 });
      const defensesScreen = await stableScreen(page, directory, "neutral-defenses-selected");
      const defensesSelected = { id: 10790, ...compareIcon(defensesScreen, setup.neutral_defenses,
        resource(source, 10790), "neutral-defenses-selected", directory) };
      assert.deepEqual([defensesRest.different_pixels, defensesSelected.different_pixels], [0, 0],
        JSON.stringify([defensesRest, defensesSelected]));
      neutral.defenses = [defensesRest, defensesSelected];
      return { compares, absent, headquarters, uprising, neutral: { ...neutral, white }, destroyed };
    },
  },
  {
    name: "selected",
    // FUN_004593e0 WM_PAINT draws a selected overlay's second bitmap, the
    // next id in FUN_0045ca80's pair; a press selects the overlay under it
    // (FUN_0045b1b0) and deselects the rest (FUN_0045afc0).
    async run(page, faction, setup, directory, source) {
      const shown = Object.entries(expectedSides(faction).primary)
        .filter(([, side]) => side !== null);
      assert.ok(shown.length >= 2, "the primary system shows two icons");
      const compares = [];
      for (const [pressed] of shown) {
        const [left, top] = quadrant(setup, "primary", pressed);
        await click(page, { x: Math.round(left) + 4, y: Math.round(top) + 9 });
        const screenshot = await stableScreen(page, directory, `selected-${pressed}`);
        for (const [name, side] of shown) {
          const id = art[name][side] + (name === pressed ? 1 : 0);
          compares.push({
            id,
            ...compareIcon(screenshot, quadrant(setup, "primary", name), resource(source, id),
              `selected-${pressed}-${name}`, directory),
          });
        }
      }
      // A right press selects too (FUN_0045b1b0 on the press, then the
      // menu on the release): A0 quadrant-select/02, 03 show the mission
      // icon's selected art after the menu's Escape, the earlier selection
      // back to normal.
      const [firstName] = shown[0];
      const [fl, ft] = quadrant(setup, "primary", firstName);
      await rightClick(page, { x: Math.round(fl) + 4, y: Math.round(ft) + 9 });
      await page.waitForTimeout(300);
      await page.keyboard.press("Escape");
      await page.waitForTimeout(300);
      const afterRight = await stableScreen(page, directory, `right-selected-${firstName}`);
      for (const [name, side] of shown) {
        const id = art[name][side] + (name === firstName ? 1 : 0);
        compares.push({
          id,
          ...compareIcon(afterRight, quadrant(setup, "primary", name), resource(source, id),
            `right-selected-${firstName}-${name}`, directory),
        });
      }
      assertExact(compares);
      return { compares };
    },
  },
  {
    name: "open-system",
    // FUN_004593e0 case 0x203 finds the shown overlay under the point;
    // FUN_0045aac0 maps kind 4 to the System window (type 9). The point lies
    // on the top-left icon, left of the planet's picture.
    async run(page, faction, setup, directory, source) {
      const [left, top] = quadrant(setup, "primary", "system");
      const at = { x: Math.round(left) + 4, y: Math.round(top) + 9 };
      await doubleClick(page, at);
      const opened = await until(page, "the system icon opens the System window",
        (o, dat) => o.system_windows.some(([system]) => system === dat), setup.primary_dat_id);
      assert.equal(opened.system_windows.length, 1, JSON.stringify(opened));
      // Minimize (0x466) sends it to the rail. FUN_00428d70 draws the
      // window's icon (FUN_00458cf0: 11530 side 1, 11531 side 2) at the
      // slot's corner, the system's name after it.
      const dat = setup.primary_dat_id;
      const [, origin] = opened.system_windows.find(([system]) => system === dat);
      const textChecks = faction.side === 1 && a0Dir ? await systemWindowText(page, origin, directory) : null;
      await click(page, { x: origin[0] + 195 + 7, y: origin[1] + 3 + 7 });
      const railed = await until(page, "minimize sends the System window to the rail",
        (o, value) => !o.system_windows.some(([system]) => system === value)
          && o.rail.some(([system, kind]) => system === value && kind === "system"), dat);
      const slot = railed.rail.find(([system, kind]) => system === dat && kind === "system")[2];
      const screenshot = await stableScreen(page, directory, "system-railed");
      const icon = faction.side === 1 ? 11530 : 11531;
      const railChecks = [{ id: icon, ...compareIcon(screenshot, slot, resource(source, icon), "system-rail-icon", directory) }];
      assertExact(railChecks);
      // The rail's slots are fixed (FUN_00428b40, FUN_00429020): the
      // Defenses window minimized next takes the second slot; restoring the
      // System window leaves its slot empty, the Defenses window where it
      // was, and a second minimize fills the gap (A0 personnel-states/32).
      await page.mouse.move(2, 2);
      await page.waitForTimeout(500);
      const defenses = await openDefenses(page, setup, "primary");
      const defensesOrigin = corner(defenses.window);
      await click(page, { x: defensesOrigin[0] + 210, y: defensesOrigin[1] + 9 });
      const both = await until(page, "the Defenses window takes the next slot",
        (o, value) => o.rail.some(([system, kind]) => system === value && kind === "defenses"), dat);
      const defensesSlot = both.rail.find(([system, kind]) => system === dat && kind === "defenses")[2];
      assert.notDeepEqual(defensesSlot, slot, JSON.stringify(both.rail));
      // The icon's double click brings the window back; its close box at
      // (209, 3) closes it, off the rail too (A0 personnel-states/31:
      // Mon Calamari's System window gone after its close box).
      const restore = async (label) => {
        // The frames after a minimize can run slow; egui pairs the two
        // clicks only within its double-click time, so let them settle.
        await page.mouse.move(2, 2);
        await frames(page, 4);
        await page.waitForTimeout(1_000);
        await doubleClick(page, at);
        return until(page, label,
          (o, value) => o.system_windows.some(([system]) => system === value)
            && !o.rail.some(([system, kind]) => system === value && kind === "system"), dat);
      };
      let back = await restore("the double click restores the System window");
      const gap = back.rail.find(([system, kind]) => system === dat && kind === "defenses");
      assert.deepEqual(gap?.[2], defensesSlot, JSON.stringify(back.rail));
      // FUN_00428d70 draws nothing in an empty slot, so the shell shows
      // (STRATEGY 900 / 901): A0 personnel-states/32's empty fifth slot is
      // 900's (544, 149, 62, 18), 0 of 1116 pixels off.
      await page.mouse.move(2, 470);
      await page.waitForTimeout(500);
      const gapScreen = await stableScreen(page, directory, "rail-gap");
      const shell = faction.side === 1 ? 900 : 901;
      const gapChecks = [{ id: shell, ...compareRegion(gapScreen, [0, 0], resource(source, shell), slot, "rail-empty-slot", directory) }];
      assertExact(gapChecks);
      let [, again] = back.system_windows.find(([system]) => system === dat);
      await click(page, { x: again[0] + 195 + 7, y: again[1] + 3 + 7 });
      const refilled = await until(page, "a second minimize fills the gap",
        (o, value) => o.rail.some(([system, kind]) => system === value && kind === "system"), dat);
      const refilledSlot = refilled.rail.find(([system, kind]) => system === dat && kind === "system")[2];
      assert.deepEqual(refilledSlot, slot, JSON.stringify(refilled.rail));
      back = await restore("the double click restores the System window again");
      [, again] = back.system_windows.find(([system]) => system === dat);
      await click(page, { x: again[0] + 209 + 7, y: again[1] + 3 + 7 });
      const closed = await until(page, "the close box closes the System window",
        (o, value) => !o.system_windows.some(([system]) => system === value)
          && !o.rail.some(([system, kind]) => system === value && kind === "system"), dat);
      await stableScreen(page, directory, "system-closed");
      return { at, opened, railChecks, textChecks, slots: { system: slot, defenses: defensesSlot }, gapChecks, closed: closed.system_windows.length };
    },
  },
  {
    name: "shared-focus",
    // FUN_00457690 draws a window's focused strip (+0x180) only while it is
    // the galaxy view's focused child, else +0x184 (FUN_00456230: side 1
    // 10299 / 10200, side 2 10201 / 10302). A0
    // 2026-11-11-manufacturing-focus/02: a Defenses window opened beside a
    // System window turns the System window's strip unfocused. One focus is
    // shared by the window kinds (window_focus.rs).
    async run(page, faction, setup, directory, source) {
      const focusedId = faction.side === 1 ? 10299 : 10201;
      const unfocusedId = faction.side === 1 ? 10200 : 10302;
      // The strip right of the title text and left of the title buttons.
      const strip = (screenshot, origin, id, label) => ({
        id,
        ...compareRegion(screenshot, [origin[0] + 2, origin[1] + 2], resource(source, id), [140, 0, 40, 16],
          label, directory),
      });
      const { dat, window } = await openDefenses(page, setup, "primary");
      // The Defenses window covers the system icon: drag it off to the
      // right by its title bar (window_move.rs), as defenses-icon-scrap does.
      const [rowLeft, rowTop] = window.tabs[0][1];
      const grab = { x: Math.round(rowLeft) + 60, y: Math.round(rowTop) - 12 };
      await page.mouse.move(grab.x, grab.y);
      await frames(page);
      await page.mouse.down();
      await frames(page, 2);
      await page.mouse.move(grab.x + 260, grab.y, { steps: 12 });
      await frames(page, 2);
      await page.mouse.up();
      await frames(page, 2);
      await page.waitForTimeout(600);
      const moved = await until(page, "the Defenses window was dragged",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value), dat);
      const defensesOrigin = corner(defensesWindow(moved, dat));
      const [left, top] = quadrant(setup, "primary", "system");
      await doubleClick(page, { x: Math.round(left) + 4, y: Math.round(top) + 9 });
      const opened = await until(page, "the system icon opens the System window",
        (o, value) => o.system_windows.some(([system]) => system === value), dat);
      const [, systemOrigin] = opened.system_windows.find(([system]) => system === dat);
      await page.mouse.move(2, 470);
      await page.waitForTimeout(500);
      let screenshot = await stableScreen(page, directory, "system-opened-last");
      const checks = [
        strip(screenshot, systemOrigin, focusedId, "system-strip-focused"),
        strip(screenshot, defensesOrigin, unfocusedId, "defenses-strip-unfocused"),
      ];
      // A press on the Defenses window's title bar gives it the focus.
      await click(page, { x: defensesOrigin[0] + 150, y: defensesOrigin[1] + 9 });
      await page.mouse.move(2, 470);
      await page.waitForTimeout(500);
      screenshot = await stableScreen(page, directory, "defenses-pressed");
      checks.push(
        strip(screenshot, systemOrigin, unfocusedId, "system-strip-unfocused"),
        strip(screenshot, defensesOrigin, focusedId, "defenses-strip-focused"),
      );
      // And a press on the System window's title bar takes it back.
      await click(page, { x: systemOrigin[0] + 150, y: systemOrigin[1] + 9 });
      await page.mouse.move(2, 470);
      await page.waitForTimeout(500);
      screenshot = await stableScreen(page, directory, "system-pressed");
      checks.push(
        strip(screenshot, systemOrigin, focusedId, "system-strip-refocused"),
        strip(screenshot, defensesOrigin, unfocusedId, "defenses-strip-unfocused-again"),
      );
      assertExact(checks);
      return { systemOrigin, defensesOrigin, checks };
    },
  },
  {
    name: "troop-finder-display",
    // The Troop Finder's Enter, Display or double click (FUN_0046df90,
    // FUN_00429440) opens the row system's Defenses window on its regiment
    // page: A0 personnel-states/15, 17, 18 (Mon Calamari, Pzob, Umgul).
    // F4 opens it with its first row chosen and the name box focused.
    async run(page, faction, setup, directory) {
      await page.mouse.move(2, 2);
      await page.keyboard.press("F4");
      await page.waitForTimeout(800);
      await page.keyboard.press("Enter");
      const shown = await until(page, "the Troop Finder's Enter opens a Defenses window",
        (o) => o.defenses_windows.length > 0);
      const window = shown.defenses_windows[0];
      assert.equal(window.page, "regiments", JSON.stringify(window));
      assert.ok(window.rows.length > 0, JSON.stringify(window));
      await stableScreen(page, directory, "troop-finder-display");
      return { window };
    },
  },
  {
    name: "defenses",
    // FUN_0045aac0 maps kind 8 to type 10 (FUN_004a7790), which opens on
    // the personnel page (FUN_0060d7e0(strip, 1)). The player's own system:
    // its regiment and KDY-150, the garrison line on the regiment page
    // (FUN_004a90d0's tail), the row frame keyed over the selected mini
    // (FUN_004a9ab0), and minimize to the rail and back (0x466,
    // FUN_004aa4a0).
    async run(page, faction, setup, directory, source) {
      const { at, dat, window } = await openDefenses(page, setup, "primary");
      assert.equal(window.side, faction.side, JSON.stringify(window));
      assert.equal(window.page, "personnel");
      // A double click opens the window at the pointer (FUN_004593e0 passes
      // its point to FUN_0045aac0), not at the icon's corner, clamped into
      // the galaxy view: A0 personnel-states/29, Mission at Pzob at x 172
      // from a double click at (172, 286), its icon starting at x 148.
      assert.equal(window.origin[0], Math.min(at.x, faction.galaxyRight - 235), JSON.stringify({ at, window }));
      assert.deepEqual(window.counts, [0, 1, 0, 0, 1]);
      assert.deepEqual([window.rows, window.selected, window.garrison], [[], null, null]);
      const origin = corner(window);
      const background = resource(source, 10577);
      let screenshot = await stableScreen(page, directory, "personnel");
      const opening = [
        ...compareTabs(screenshot, source, window, directory, "personnel"),
        // The empty list and the strip below it show the background.
        { id: 10577, ...compareRegion(screenshot, origin, background, [7, 81, 222, 210], "personnel-list", directory) },
        { id: 10577, ...compareRegion(screenshot, origin, background, [0, 292, 235, 12], "personnel-bottom", directory) },
      ];
      assertExact(opening);
      // FUN_004a8790: every tab has its tip, none is ever disabled.
      for (const [name, rect] of window.tabs) {
        assert.ok(await tipShows(page, { x: Math.round(rect[0] + 18), y: Math.round(rect[1] + 16) }),
          `the ${name} tab shows no tip`);
      }

      // The Shields and Fighters pages' labels (TEXTSTRA "Planetary
      // Shields", "Fighter Squadrons", font 5 centred at the page's top)
      // against A0 status/17 and 19 (Coruscant's window at (196, 91)): each
      // label's ink box, relative to the window, left and top within 2
      // pixels, width within 15% (the accepted text deviation).
      const pageLabels = {};
      const labelBox = (png, [ox, oy], bottom = 70) => {
        const white = (x, y) => {
          const i = (y * png.width + x) * 4;
          return Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) > 215;
        };
        const cols = [];
        for (let x = ox + 30; x < ox + 210; x += 1) {
          let n = 0;
          for (let y = oy + 52; y < oy + Math.min(68, bottom); y += 1) if (white(x, y)) n += 1;
          if (n >= 2) cols.push(x - ox);
        }
        const rows = [];
        for (let y = oy + 50; y < oy + bottom; y += 1) {
          let n = 0;
          for (let x = ox + 30; x < ox + 210; x += 1) if (white(x, y)) n += 1;
          if (n >= 3) rows.push(y - oy);
        }
        return cols.length && rows.length ? [cols[0], rows[0], cols.at(-1), rows.at(-1)] : null;
      };
      // A page's label against an A0 window whose corner is `a0Origin`.
      const checkLabel = (name, shot, a0File, a0Origin, bottom = 70) => {
        if (!fs.existsSync(a0File)) return;
        const a = labelBox(PNG.sync.read(fs.readFileSync(a0File)), a0Origin, bottom);
        const p = labelBox(shot, origin, bottom);
        pageLabels[name] = { a0: a, port: p };
        const ratio = a && p ? (p[2] - p[0] + 1) / (a[2] - a[0] + 1) : 0;
        assert.ok(a && p && a[2] - a[0] < 150 && Math.abs(a[0] - p[0]) <= 2 && Math.abs(a[1] - p[1]) <= 2
          && ratio > 0.85 && ratio < 1.15, `${name} page label ${JSON.stringify(pageLabels[name])}`);
      };
      // "Personnel" against A0 personnel-states/16 (Pzob's window at
      // (118, 86)).
      checkLabel("personnel", screenshot,
        path.join(root, ".artifacts/a0-vm/personnel-states/16-pzob-personnel.png"), [118, 86]);
      for (const [name, file] of [["shields", "17-def-tab.png"], ["squadrons", "19-tab3.png"]]) {
        const tab = window.tabs.find(([tabName]) => tabName === name)[1];
        // The tip loop left a tip up; move off before the press.
        await page.mouse.move(2, 2);
        await page.waitForTimeout(300);
        await click(page, { x: tab[0] + 18, y: tab[1] + 16 });
        await until(page, `the ${name} tab opens its page`,
          (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value[0] && w.page === value[1]),
          [dat, name]);
        const shot = await stableScreen(page, directory, `${name}-page`);
        const a0File = path.join(root, ".artifacts/a0-vm/status", file);
        if (fs.existsSync(a0File)) {
          const a = labelBox(PNG.sync.read(fs.readFileSync(a0File)), [196, 91]);
          const p = labelBox(shot, origin);
          pageLabels[name] = { a0: a, port: p };
          const ratio = a && p ? (p[2] - p[0] + 1) / (a[2] - a[0] + 1) : 0;
          assert.ok(a && p && a[2] - a[0] < 150 && Math.abs(a[0] - p[0]) <= 2 && Math.abs(a[1] - p[1]) <= 2
            && ratio > 0.85 && ratio < 1.15, `${name} page label ${JSON.stringify(pageLabels[name])}`);
        }
      }

      const regimentsTab = window.tabs.find(([name]) => name === "regiments")[1];
      await click(page, { x: regimentsTab[0] + 18, y: regimentsTab[1] + 16 });
      const regiments = defensesWindow(await until(page, "the regiment tab opens its page",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value && w.page === "regiments"), dat), dat);
      assert.equal(regiments.rows.length, 1, JSON.stringify(regiments));
      assert.match(regiments.garrison, /^Garrison Requirement: \d+$/);
      screenshot = await stableScreen(page, directory, "regiments");
      // "Trooper Regiments" against A0 personnel-states/15 (Mon Calamari's
      // window at (305, 86)), its first line only. That save builds its
      // messages for the Empire behind an Alliance cockpit, so it showed no
      // garrison line on the Alliance's systems.
      await page.mouse.move(2, 2);
      await page.waitForTimeout(300);
      const regimentsLabel = await stableScreen(page, directory, "regiments-label");
      checkLabel("regiments", regimentsLabel,
        path.join(root, ".artifacts/a0-vm/personnel-states/15-mon-calamari-regiments.png"), [305, 86], 66);
      // The garrison line (FUN_004a90d0's tail for the player's side's
      // system) against an unedited Alliance game's own Yavin (A0
      // garrison-clean/test-day483-yavin-regiments, window at (305, 86):
      // "Garrison Requirement: 0"): its ink box's left and top within 2
      // pixels, its width within 15% (the accepted text deviation).
      const garrisonA0 = path.join(root, ".artifacts/a0-vm/garrison-clean/test-day483-yavin-regiments.png");
      if (faction.side === 1 && fs.existsSync(garrisonA0)) {
        const garrisonBox = (png, [ox, oy]) => {
          let box = null;
          for (let y = oy + 66; y < oy + 80; y += 1) {
            for (let x = ox + 30; x < ox + 210; x += 1) {
              const i = (y * png.width + x) * 4;
              if (Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) <= 215) continue;
              box = box ? [Math.min(box[0], x - ox), Math.min(box[1], y - oy), Math.max(box[2], x - ox),
                Math.max(box[3], y - oy)] : [x - ox, y - oy, x - ox, y - oy];
            }
          }
          return box;
        };
        const a = garrisonBox(PNG.sync.read(fs.readFileSync(garrisonA0)), [305, 86]);
        const p = garrisonBox(regimentsLabel, origin);
        pageLabels.garrison = { a0: a, port: p };
        const ratio = a && p ? (p[2] - p[0] + 1) / (a[2] - a[0] + 1) : 0;
        assert.ok(a && p && Math.abs(a[0] - p[0]) <= 2 && Math.abs(a[1] - p[1]) <= 2 && ratio > 0.85 && ratio < 1.15,
          `the garrison line ${JSON.stringify(pageLabels.garrison)}`);
      }
      // The whole window against the same A0 Yavin (305, 86): every pixel
      // but the title, the page name and garrison lines (text), the first
      // two cells (A0's two Army regiments, the fixture's one Fleet
      // regiment) and the tabs whose empty art differs by state (A0's Yavin
      // lists personnel and squadrons but no batteries; compareTabs checks
      // each tab against its bitmap). Their bottom row, with the tab's key
      // pixel, stays in: A0 shows 10577's white there (FUN_00602d30 ->
      // FUN_005fd0f0), and black under the title buttons' key pixels.
      if (faction.side === 1 && fs.existsSync(garrisonA0)) {
        pageLabels.whole = wholeWindow(regimentsLabel, origin, garrisonA0, [305, 86], [
          [19, 2, 175, 14], [2, 53, 231, 27], [7, 81, 140, 70],
          [28, 20, 36, 32], [100, 20, 36, 32], [172, 20, 36, 32],
        ]);
        assert.ok(pageLabels.whole.compared > 45_000 && pageLabels.whole.differing === 0,
          `the whole Defenses window: ${JSON.stringify(pageLabels.whole)}`);
      }
      const mini = REGIMENT_MINI[faction.side];
      const regimentChecks = [
        ...compareTabs(screenshot, source, regiments, directory, "regiments"),
        { id: mini, ...compareIcon(screenshot, regiments.cells[0], gokres(mini), "regiments-mini", directory) },
      ];
      assertExact(regimentChecks);

      const cell = regiments.cells[0];
      await click(page, { x: cell[0] + 30, y: cell[1] + 12 });
      const selected = defensesWindow(await until(page, "a click selects the row",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value && w.selected === 0), dat), dat);
      screenshot = await stableScreen(page, directory, "selected");
      const frame = faction.side === 1 ? 10578 : 10579;
      const selectedChecks = [
        { id: frame, ...compareIcon(screenshot, selected.cells[0], resource(source, frame), "selected-frame", directory) },
      ];
      assertExact(selectedChecks);

      await click(page, { x: origin[0] + 210, y: origin[1] + 9 });
      const railed = await until(page, "minimize sends the window to the rail",
        (o, value) => !o.defenses_windows.some((w) => w.system_dat_id === value)
          && o.rail.some(([system, kind]) => system === value && kind === "defenses"), dat);
      const slot = railed.rail.find(([system, kind]) => system === dat && kind === "defenses")[2];
      screenshot = await stableScreen(page, directory, "railed");
      const icon = faction.side === 1 ? 11533 : 11534;
      const railChecks = [{ id: icon, ...compareIcon(screenshot, slot, resource(source, icon), "rail-icon", directory) }];
      assertExact(railChecks);

      // The rail slot restores on a double click (FUN_00422ce0 0x203).
      await doubleClick(page, { x: slot[0] + slot[2] / 2, y: slot[1] + slot[3] / 2 });
      const restored = defensesWindow(await until(page, "the rail slot restores the window",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value)
          && !o.rail.some(([system, kind]) => system === value && kind === "defenses"), dat), dat);
      assert.deepEqual(restored.origin, window.origin);
      // A minimized window is hidden, not destroyed: it comes back on its
      // page, and its icon's double click takes it off the rail too (A0
      // personnel-states/38, Mon Calamari's Trooper Regiments).
      assert.equal(restored.page, "regiments", JSON.stringify(restored));
      await page.mouse.move(2, 2);
      await page.waitForTimeout(500);
      await click(page, { x: origin[0] + 210, y: origin[1] + 9 });
      await until(page, "minimize sends the window to the rail again",
        (o, value) => !o.defenses_windows.some((w) => w.system_dat_id === value)
          && o.rail.some(([system, kind]) => system === value && kind === "defenses"), dat);
      await page.mouse.move(2, 2);
      await frames(page, 4);
      await page.waitForTimeout(1_000);
      await doubleClick(page, at);
      const fromIcon = defensesWindow(await until(page, "the icon takes the window off the rail",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value)
          && !o.rail.some(([system, kind]) => system === value && kind === "defenses"), dat), dat);
      assert.deepEqual([fromIcon.page, fromIcon.origin], ["regiments", window.origin], JSON.stringify(fromIcon));
      return { at, window, opening, regiments, regimentChecks, selectedChecks, railChecks, restored, fromIcon, pageLabels };
    },
  },
  {
    name: "system-menu",
    // A press on a planet hits no overlay, so FUN_004593e0 returns and the
    // window's selection is kept (sector-icon-menus.md). The planet is the
    // system in uprising's: the headquarters' mark 904 is an overlay
    // (FUN_0045cc10), opaque at its planet's centre. The right release
    // lists that selection's orders (FUN_004ac5c0, FUN_0051d990). With
    // nothing selected, Encyclopedia (one object) and Status are both
    // disabled. After a press selects the primary's mission icon, a right
    // click on another planet shows the mission icon's menu, as A0
    // menus/10-planet-right.png shows the kept fleet icon's menu.
    async run(page, faction, setup, directory) {
      const [left, top, width, height] = setup.uprising;
      const planet = { x: Math.round(left + width / 2), y: Math.round(top + height / 2) };
      const openMenu = async (label) => {
        await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
        await rightClick(page, planet);
        await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
          null, { timeout: 10_000 });
        const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
        return { menu, screenshot: await stableScreen(page, directory, label) };
      };
      // Rows are 20 pixels below a 2-pixel border; the text line is the 16
      // pixels from 2 into the row (game_menu.rs). Disabled text is
      // 0x808080 (FUN_004aba60).
      const brightest = ({ menu, screenshot }, row) => {
        const y0 = Math.round(menu.top) + 2 + 20 * row + 2;
        let value = 0;
        for (let y = y0; y < y0 + 16; y += 1) {
          for (let x = Math.round(menu.left) + 4; x < menu.left + menu.width - 3; x += 1) {
            const at = (y * screenshot.width + x) * 4;
            value = Math.max(value, screenshot.data[at], screenshot.data[at + 1], screenshot.data[at + 2]);
          }
        }
        return value;
      };
      const empty = await openMenu("no-selection");
      assert.equal(empty.menu.rows, 2, JSON.stringify(empty.menu));
      assert.equal(empty.menu.status_window_row, 1, JSON.stringify(empty.menu));
      const emptyRows = [brightest(empty, 0), brightest(empty, 1)];
      assert.deepEqual(emptyRows, [128, 128], `nothing selected: ${JSON.stringify(emptyRows)}`);
      await page.keyboard.press("Escape");
      await frames(page, 2);

      const [missionLeft, missionTop] = quadrant(setup, "primary", "missions");
      await click(page, { x: Math.round(missionLeft) + 4, y: Math.round(missionTop) + 9 });
      const kept = await openMenu("kept-selection");
      assert.equal(kept.menu.rows, 3, JSON.stringify(kept.menu));
      assert.notEqual(kept.menu.abort_row, null, `the mission icon's Abort: ${JSON.stringify(kept.menu)}`);
      // Encyclopedia is enabled for the one selected icon; an icon's
      // Status is always disabled.
      const keptRows = [brightest(kept, 0), brightest(kept, 1)];
      assert.deepEqual(keptRows, [255, 128], `the kept icon: ${JSON.stringify(keptRows)}`);
      return { empty: empty.menu, emptyRows, kept: kept.menu, keptRows };
    },
  },
  {
    name: "headquarters-menu",
    // The headquarters overlay (id 0x10000, FUN_0045bbb0) sorts before
    // every quadrant icon (FUN_0060a790(1)), so FUN_0045cc10 gives a press
    // on 904's opaque pixels to it. Its object is the HQ (ALLFACSD, 0x20),
    // whose slot +0x3c (FUN_00526860 -> FUN_00526c00) lists Move and
    // Confirmed Move; FUN_0051d990 adds Encyclopedia and Status. Status
    // opens FUN_00444e20's Defense Facility Status (families 0x20..0x27):
    // Location, Status, Maintenance Cost and Bombardment Defense Strength.
    // The manual (p. 134): "right-click, select Move"; FUN_00578c00 enables
    // both moves for the HQ at rest. The Empire does not see this
    // headquarters (the icons case), so its press selects nothing.
    async run(page, faction, setup, directory) {
      const [left, top] = setup.headquarters;
      // 904's (20, 12) is opaque, in the fleet icon's quadrant.
      const at = { x: Math.round(left) + 20, y: Math.round(top) + 12 };
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
      await rightClick(page, at);
      await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
        null, { timeout: 10_000 });
      const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
      await stableScreen(page, directory, "headquarters-menu");
      if (faction.side !== 1) {
        assert.deepEqual(menu.row_states, [["Encyclopedia", false], ["Status", false]], JSON.stringify(menu));
        return { at, menu };
      }
      assert.deepEqual(menu.row_states,
        [["Move", true], ["Confirmed Move", true], ["Encyclopedia", true], ["Status", true]], JSON.stringify(menu));
      // A0 headquarters/01-hq-menu (Pzob): 104 by 82 from the cursor, the
      // same four rows, all white.
      assert.deepEqual([menu.left, menu.top, menu.width, menu.height], [at.x, at.y, 104, 82], JSON.stringify(menu));
      const height = (menu.height - 2) / menu.rows;
      await page.evaluate(() => { window.__openRebellionInterfaceStatusRows = undefined; });
      await click(page, { x: menu.left + menu.width / 2, y: menu.top + 2 + height * (menu.status_window_row + 0.5) });
      await page.waitForFunction(() => window.__openRebellionInterfaceStatusRows?.rows?.length, null, { timeout: 10_000 });
      const status = await page.evaluate(() => window.__openRebellionInterfaceStatusRows);
      await page.mouse.move(2, 2);
      const screen = await stableScreen(page, directory, "headquarters-status");
      assert.equal(status.title, "Defense Facility Status", JSON.stringify(status));
      // A0 headquarters/02-hq-status (Pzob, day 446, the window at (108,
      // 79), as here): the picture frame and the buttons match exactly;
      // the title, the five label lines, the name and three values each
      // ink within a pixel of A0's (the accepted text deviation). The
      // Location value names another system and is skipped.
      let a0 = null;
      const a0File = path.join(root, ".artifacts/a0-vm/headquarters/02-hq-status.png");
      if (fs.existsSync(a0File)) {
        const shot = PNG.sync.read(fs.readFileSync(a0File));
        const [x0, y0] = [108, 79];
        const pixel = (png, x, y) => Array.from(png.data.subarray(((y0 + y) * png.width + x0 + x) * 4, ((y0 + y) * png.width + x0 + x) * 4 + 3));
        const differing = ([xa, ya, xb, yb]) => {
          let count = 0;
          for (let y = ya; y < yb; y += 1) for (let x = xa; x < xb; x += 1) {
            if (pixel(shot, x, y).some((value, k) => value !== pixel(screen, x, y)[k])) count += 1;
          }
          return count;
        };
        const ink = (png, [xa, ya, xb, yb]) => {
          let box = null;
          for (let y = ya; y < yb; y += 1) for (let x = xa; x < xb; x += 1) {
            if (Math.min(...pixel(png, x, y)) <= 200) continue;
            box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
          }
          return box;
        };
        a0 = { picture: differing([242, 15, 372, 113]), buttons: differing([242, 200, 372, 260]), ink: {} };
        assert.deepEqual([a0.picture, a0.buttons], [0, 0], JSON.stringify(a0));
        const regions = { title: [15, 18, 226, 36], name: [242, 137, 372, 181] };
        for (let k = 0; k < 5; k += 1) regions[`label-${k}`] = [16, 47 + 14 * k, 118, 61 + 14 * k];
        for (const k of [1, 2, 4]) regions[`value-${k}`] = [119, 47 + 14 * k, 210, 61 + 14 * k];
        for (const [name, region] of Object.entries(regions)) {
          const [a, b] = [ink(shot, region), ink(screen, region)];
          a0.ink[name] = { a0: a, port: b };
          assert.ok(a && b && a.every((value, k) => Math.abs(value - b[k]) <= 1), `${name}: ${JSON.stringify(a0.ink[name])}`);
        }
      }
      assert.deepEqual(status.rows.map(([label]) => label),
        ["Location:", "Status:", "Maintenance Cost:", "Bombardment Defense Strength:"], JSON.stringify(status));
      // ALLFACSD 1's last word, class +0x58 (FUN_00520b60 via vtable
      // 0x0065f280 +0x1c0), is 9.
      assert.deepEqual([status.rows[1][1], status.rows[3][1]], ["Active", "9"], JSON.stringify(status));
      return { at, menu, status, a0 };
    },
  },
  {
    name: "headquarters-move",
    // Move (0x201) of the HQ, then a click on another of the side's
    // systems (the manual, p. 134). The command moves the object at once
    // and marks it en route (FUN_00556390): 904 now stands over the
    // destination (FUN_0045bbb0, mode 3 "existing"; port: redrawn at once,
    // where A0 headquarters-move/09 keeps an open window's mark at the
    // origin until the window reopens), the HQ's menu greys
    // both moves (FUN_004f9860 4), and its Status leads with the ETA
    // (FUN_004fd2b0) and reads "Enroute" (FUN_0044a470). The Empire sees no
    // headquarters here.
    async run(page, faction, setup, directory, source) {
      if (faction.side !== 1) return { skipped: "the Empire sees no headquarters" };
      const [left, top] = setup.headquarters;
      const [toLeft, toTop] = setup.uprising;
      const at = { x: Math.round(left) + 20, y: Math.round(top) + 12 };
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
      await rightClick(page, at);
      await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
        null, { timeout: 10_000 });
      const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
      assert.deepEqual([menu.move_row, menu.row_states[menu.move_row]], [0, ["Move", true]], JSON.stringify(menu));
      const height = (menu.height - 2) / menu.rows;
      await click(page, { x: menu.left + menu.width / 2, y: menu.top + 2 + height * (menu.move_row + 0.5) });
      await frames(page, 2);
      await click(page, { x: Math.round(toLeft) + 18, y: Math.round(toTop) + 18 });
      await page.waitForTimeout(600);
      await page.mouse.move(2, 2);
      const screen = await stableScreen(page, directory, "headquarters-moved");
      const icon = resource(source, 904);
      const gone = compareIcon(screen, setup.headquarters, icon, "headquarters-origin", directory);
      const arrived = compareIcon(screen, setup.uprising, icon, "headquarters-destination", directory);
      assert.ok(gone.different_pixels > gone.pixels_checked / 2, `904 left its origin: ${JSON.stringify(gone)}`);
      assert.equal(arrived.different_pixels, 0, `904 over the destination: ${JSON.stringify(arrived)}`);
      const there = { x: Math.round(toLeft) + 20, y: Math.round(toTop) + 12 };
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
      await rightClick(page, there);
      await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
        null, { timeout: 10_000 });
      const enRoute = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
      await stableScreen(page, directory, "headquarters-en-route-menu");
      assert.deepEqual(enRoute.row_states,
        [["Move", false], ["Confirmed Move", false], ["Encyclopedia", true], ["Status", true]], JSON.stringify(enRoute));
      const rowHeight = (enRoute.height - 2) / enRoute.rows;
      await page.evaluate(() => { window.__openRebellionInterfaceStatusRows = undefined; });
      await click(page, { x: enRoute.left + enRoute.width / 2, y: enRoute.top + 2 + rowHeight * (enRoute.status_window_row + 0.5) });
      await page.waitForFunction(() => window.__openRebellionInterfaceStatusRows?.rows?.length, null, { timeout: 10_000 });
      const status = await page.evaluate(() => window.__openRebellionInterfaceStatusRows);
      await page.mouse.move(2, 2);
      await stableScreen(page, directory, "headquarters-en-route-status");
      assert.equal(status.title, "Defense Facility Status", JSON.stringify(status));
      assert.deepEqual(status.rows.map(([label]) => label),
        ["ETA Destination:", "Location:", "Status:", "Maintenance Cost:", "Bombardment Defense Strength:"],
        JSON.stringify(status));
      assert.match(status.rows[0][1], /^Day \d+$/, JSON.stringify(status));
      assert.equal(status.rows[2][1], "Enroute", JSON.stringify(status));
      return { menu, gone, arrived, enRoute, status };
    },
  },
  {
    name: "battery-menu",
    // A battery's pop-up menu: the facility's order list is FUN_00539c20's
    // one 0x200 Scrap (FUN_005399e0), with FUN_0051d990's Encyclopedia and
    // Status, sorted Encyclopedia, Status, Scrap and all enabled for the
    // player's own battery, as A0 status/22-battery-menu.png shows.
    async run(page, faction, setup, directory) {
      const { dat, window } = await openDefenses(page, setup, "primary");
      const batteriesTab = window.tabs.find(([name]) => name === "batteries")[1];
      await click(page, { x: batteriesTab[0] + 18, y: batteriesTab[1] + 16 });
      const batteries = defensesWindow(await until(page, "the battery tab opens its page",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value && w.page === "batteries"), dat), dat);
      assert.equal(batteries.cells.length, 1, JSON.stringify(batteries));
      const cell = batteries.cells[0];
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
      await rightClick(page, { x: cell[0] + 20, y: cell[1] + 12 });
      await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
        null, { timeout: 10_000 });
      const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
      await stableScreen(page, directory, "battery-menu");
      assert.equal(menu.rows, 3, JSON.stringify(menu));
      assert.deepEqual([menu.status_window_row, menu.scrap_row, menu.scrap_enabled], [1, 2, true],
        JSON.stringify(menu));
      assert.deepEqual([menu.move_row, menu.confirmed_move_row], [null, null], JSON.stringify(menu));
      return { batteries, menu };
    },
  },
  {
    name: "defenses-icon-scrap",
    // The defenses icon's Scrap (0x200) acts on the system's regiments,
    // squadrons and defense facilities of the player's side (FUN_00512700
    // kind 8, FUN_00553350): A0 menus3/11-12 enable it on Coruscant and ask
    // with each regiment's name. The fixture's primary system holds one
    // regiment and one battery; the checkmark scraps both.
    async run(page, faction, setup, directory) {
      const { dat, window } = await openDefenses(page, setup, "primary");
      assert.deepEqual([window.counts[1], window.counts[4]], [1, 1], JSON.stringify(window));
      // The window covers the icon: drag it by its title bar off to the
      // right (window_move.rs, +0xb8 bit 4).
      const [rowLeft, rowTop] = window.tabs[0][1];
      const grab = { x: Math.round(rowLeft) + 60, y: Math.round(rowTop) - 12 };
      await page.mouse.move(grab.x, grab.y);
      await frames(page);
      await page.mouse.down();
      await frames(page, 2);
      await page.mouse.move(grab.x + 260, grab.y, { steps: 12 });
      await frames(page, 2);
      await page.mouse.up();
      await frames(page, 2);
      const [left, top] = quadrant(setup, "primary", "defenses");
      await page.waitForTimeout(600);
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
      await rightClick(page, { x: Math.round(left) + 4, y: Math.round(top) + 9 });
      await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
        null, { timeout: 10_000 });
      const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
      await stableScreen(page, directory, "defenses-icon-menu");
      assert.deepEqual(menu.row_states.map(([label]) => label),
        ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"], JSON.stringify(menu));
      assert.equal(menu.scrap_enabled, true, JSON.stringify(menu));
      const height = (menu.height - 2) / menu.rows;
      await click(page, { x: menu.left + menu.width / 2, y: menu.top + 2 + height * (menu.scrap_row + 0.5) });
      await page.waitForTimeout(400);
      await stableScreen(page, directory, "defenses-icon-scrap-confirmation");
      const { galaxy } = faction.name === "alliance"
        ? { galaxy: { x: 55, y: 40, width: 485, height: 350 } }
        : { galaxy: { x: 120, y: 40, width: 480, height: 355 } };
      await click(page, {
        x: Math.round(galaxy.x + galaxy.width / 2 - 424 / 2 + 355 + 25.5),
        y: Math.round(galaxy.y + galaxy.height / 2 - 331 / 2 + 244 + 17.5),
      });
      const after = defensesWindow(await until(page, "the checkmark scraps the regiment and the battery",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value
          && w.counts[1] === 0 && w.counts[4] === 0), dat), dat);
      await stableScreen(page, directory, "defenses-icon-scrapped");
      return { menu, before: window.counts, after: after.counts };
    },
  },
  {
    name: "defenses-icon-menu-a0",
    // The defenses icon's menu against A0
    // 2026-11-11-icon-menu-topic/00-coruscant-defenses-menu-move-grey (the
    // Empire, day 12, the menu at (203, 237)): Move and Confirmed Move
    // grey, Encyclopedia white, Status grey (an icon's Status is
    // disabled), Scrap enabled (FUN_00512700 kind 8, FUN_0051d990). Here
    // the moves are grey because the fixture's team holds the player's
    // agent on a mission (FUN_0054c110 refuses it; A0 00's cause is
    // inferred to be deliveries on their way). The pointer went over
    // Scrap and off the menu, and Scrap stays in the side's colour: FUN_004aab50 paints the item +0x118 holds, which only
    // a WM_MOUSEMOVE inside the window changes. Against A0, relative to
    // the menu's corner: its size, the frame's pixels (FUN_004aab50 over
    // GID art), each row's ink within a pixel and its colour. The fill
    // shades another scene and is not compared.
    async run(page, faction, setup, directory) {
      const [left, top] = quadrant(setup, "primary", "defenses");
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
      await rightClick(page, { x: Math.round(left) + 4, y: Math.round(top) + 9 });
      await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
        null, { timeout: 10_000 });
      const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
      assert.deepEqual(menu.row_states,
        [["Move", false], ["Confirmed Move", false], ["Encyclopedia", true], ["Status", false], ["Scrap", true]],
        JSON.stringify(menu));
      const [mx, my] = [Math.round(menu.left), Math.round(menu.top)];
      assert.deepEqual([menu.width, menu.height], [104, 102], JSON.stringify(menu));
      await page.mouse.move(mx + 50, my + 2 + 20 * 4 + 10);
      await frames(page, 2);
      await page.waitForTimeout(300);
      await frames(page, 2);
      const onScrap = PNG.sync.read((await shot(page, directory, "defenses-icon-menu-pointer-on-scrap")).bytes);
      const screen = await stableScreen(page, directory, "defenses-icon-menu-pointer-off");
      const highlight = faction.side === 1 ? [255, 0, 0] : [0, 255, 0];
      const pixel = (png, x, y) => Array.from(png.data.subarray((y * png.width + x) * 4, (y * png.width + x) * 4 + 3));
      // Each row's ink (any channel above 100: the shaded fill stays at or
      // below 92) and its commonest ink colour.
      const rowInk = (png, x0, y0, row) => {
        let box = null;
        const colours = new Map();
        for (let y = y0 + 2 + 20 * row; y < Math.min(y0 + 22 + 20 * row, y0 + 100); y += 1) {
          for (let x = x0 + 3; x < x0 + 101; x += 1) {
            const value = pixel(png, x, y);
            if (Math.max(...value) <= 100) continue;
            box = box ? [Math.min(box[0], x - x0), Math.min(box[1], y - y0), Math.max(box[2], x - x0), Math.max(box[3], y - y0)]
              : [x - x0, y - y0, x - x0, y - y0];
            colours.set(value.join(","), (colours.get(value.join(",")) ?? 0) + 1);
          }
        }
        const colour = [...colours.entries()].sort((a, b) => b[1] - a[1])[0]?.[0];
        // The colour's class: white (+0x30), grey 0x808080 within a step
        // (+0x34; the accepted palette-step deviation) or the side's
        // highlight (+0x2c, FUN_004aba60).
        const [r, g, b] = (colour ?? "").split(",").map(Number);
        const kind = colour === highlight.join(",") || colour === "0,255,0" || colour === "255,0,0" ? "highlight"
          : r === g && g === b && r >= 250 ? "white"
            : r === g && g === b && Math.abs(r - 128) <= 1 ? "grey" : colour;
        return { box, colour, kind };
      };
      const port = [0, 1, 2, 3, 4].map((row) => rowInk(screen, mx, my, row));
      // While the pointer is on Scrap, Scrap is lit too.
      assert.equal(rowInk(onScrap, mx, my, 4).colour, highlight.join(","), "Scrap under the pointer");
      assert.equal(port[4].colour, highlight.join(","), `Scrap after the pointer left: ${JSON.stringify(port)}`);
      assert.deepEqual(port.map(({ kind }) => kind),
        ["grey", "grey", "white", "grey", "highlight"], JSON.stringify(port));
      let a0 = null;
      const a0File = path.join(root, ".artifacts/a0-vm/2026-11-11-icon-menu-topic/00-coruscant-defenses-menu-move-grey.png");
      if (faction.side === 2 && fs.existsSync(a0File)) {
        const shot = PNG.sync.read(fs.readFileSync(a0File));
        const [ax, ay] = [203, 237];
        const rows = [0, 1, 2, 3, 4].map((row) => rowInk(shot, ax, ay, row));
        assert.deepEqual(rows.map(({ kind }) => kind), port.map(({ kind }) => kind), JSON.stringify(rows));
        // Each row's ink within a pixel of A0's; the right edge within two,
        // the accepted text-width deviation (Liberation Sans glyphs on
        // GDI's Arial advances; user decision 2026-10-30): "Encyclopedia"
        // inks 2 pixels wider than A0's.
        const inkDiffs = rows.map(({ box }, row) => box && port[row].box
          ? box.map((value, k) => port[row].box[k] - value) : null);
        assert.ok(inkDiffs.every((d) => d && d.every((value, k) => Math.abs(value) <= (k === 2 ? 2 : 1))),
          `rows' ink against A0: ${JSON.stringify({ a0: rows.map(({ box }) => box), port: port.map(({ box }) => box), inkDiffs })}`);
        // The frame: the two-pixel ring, where a pixel is frame art when
        // it is grey 128, 192 or 255 (the fill never reaches 100).
        const frameArt = (value) => value[0] === value[1] && value[1] === value[2] && [128, 192, 255].includes(value[0]);
        let ring = 0;
        const differing = [];
        for (let y = 0; y < 102; y += 1) {
          for (let x = 0; x < 104; x += 1) {
            if (x > 1 && x < 102 && y > 1 && y < 100) continue;
            ring += 1;
            const [a, b] = [pixel(shot, ax + x, ay + y), pixel(screen, mx + x, my + y)];
            if (frameArt(a) !== frameArt(b) || (frameArt(a) && a.join() !== b.join())) differing.push([x, y, a, b]);
          }
        }
        a0 = { rows, inkDiffs, ring, differing: differing.length };
        assert.equal(differing.length, 0, `the frame differs from A0 at ${JSON.stringify(differing.slice(0, 8))}`);
      }
      return { menu, port, a0 };
    },
  },
  {
    name: "fleet-icon-drag-image",
    scenario: QUADRANTS_NEUTRAL_FLEET,
    // FUN_004593e0 case 0x200: the first move with the button held starts
    // the drag image (FUN_0060dbb0, FUN_0060dcb0, FUN_0060dce0), the
    // window-sized bitmap FUN_0045b1b0 blitted the icon's first bitmap
    // (+0x20) into, moved with the pointer (FUN_0060df00); the icon keeps
    // its pressed art. A0 2026-11-13-icon-drag-move/20, 21: Jerijador's
    // fleet icon pressed at (266, 214), 10776 at (249, 213), and walked
    // until 10775 shows at (199, 157): the image is the icon's normal art
    // moved by (-50, -56) (the arrow covers some of its pixels there). Case
    // 0x202 past four pixels posts 0x29a, and the fleet's Move departs
    // without asking (FUN_00487cc0; A0 22: 10775 at Kubindi (219, 139), no
    // confirmation). Here the release is on the primary system's planet;
    // the fleet is shown there at once and its menu greys Move.
    async run(page, faction, setup, directory, source) {
      const [left, top] = setup.neutral_fleets;
      const [dx, dy] = [-50, -56];
      const press = { x: Math.round(left) + 17, y: Math.round(top) + 5 };
      const normal = resource(source, art.fleets[faction.side]);
      const pressed = resource(source, art.fleets[faction.side] + 1);
      // A0: the offset between the pressed origin and the drag image, and
      // each one's differing pixels (the image's under the arrow left out).
      let a0 = null;
      const start = a0Capture("2026-11-13-icon-drag-move/20-fleet-icon-drag-start.png");
      const held = a0Capture("2026-11-13-icon-drag-move/21-fleet-icon-dragged-over-kubindi.png");
      const dropped = a0Capture("2026-11-13-icon-drag-move/22-fleet-icon-released-on-kubindi.png");
      if (start && held && dropped && faction.side === 1) {
        const pointer = [266 + dx, 214 + dy];
        const arrow = (x, y) => x >= pointer[0] && y >= pointer[1] && x < pointer[0] + 12 && y < pointer[1] + 20;
        a0 = {
          pressed: templateMatch(start, pressed, [230, 280], [190, 230]),
          held_origin: templateMatch(held, pressed, [230, 280], [190, 230]),
          held_image: templateMatch(held, normal, [180, 240], [140, 180], arrow),
          dropped: templateMatch(dropped, normal, [190, 240], [130, 180]),
        };
        assert.deepEqual([a0.pressed.x, a0.pressed.y, a0.pressed.different], [249, 213, 0], JSON.stringify(a0));
        assert.deepEqual([a0.held_origin.x, a0.held_origin.y, a0.held_origin.different], [249, 213, 0], JSON.stringify(a0));
        assert.deepEqual([a0.held_image.x - a0.held_origin.x, a0.held_image.y - a0.held_origin.y, a0.held_image.different],
          [dx, dy, 0], JSON.stringify(a0));
        assert.deepEqual([a0.dropped.x, a0.dropped.y, a0.dropped.different], [219, 139, 0], JSON.stringify(a0));
      }
      await page.mouse.move(press.x, press.y);
      await frames(page);
      await page.mouse.down();
      await frames(page, 2);
      await page.waitForTimeout(300);
      await page.mouse.move(press.x + dx, press.y + dy, { steps: 10 });
      await frames(page, 2);
      await page.waitForTimeout(300);
      await frames(page, 2);
      const screen = PNG.sync.read((await shot(page, directory, "fleet-icon-held")).bytes);
      const origin = compareIcon(screen, [left, top], pressed, "fleet-icon-origin-pressed", directory);
      const image = compareIcon(screen, [left + dx, top + dy], normal, "fleet-icon-drag-image", directory);
      assert.equal(origin.different_pixels, 0, `the origin keeps its pressed art: ${JSON.stringify(origin)}`);
      assert.equal(image.different_pixels, 0, `the drag image: ${JSON.stringify(image)}`);
      // Release on the primary system's planet (its fleet icon's quadrant
      // is the planet's top right: the planet's centre lies 19.5 below and
      // half a pixel left of its corner).
      const [toLeft, toTop] = quadrant(setup, "primary", "fleets");
      const to = { x: Math.round(toLeft), y: Math.round(toTop) + 19 };
      await page.mouse.move(to.x, to.y, { steps: 12 });
      await frames(page, 2);
      await page.waitForTimeout(900);
      await page.mouse.up();
      await frames(page, 2);
      await page.waitForTimeout(600);
      const after = await stableScreen(page, directory, "fleet-icon-dropped");
      const arrived = compareIcon(after, [toLeft, toTop], normal, "fleet-icon-destination", directory);
      const gone = compareIcon(after, [left, top], normal, "fleet-icon-origin-after", directory);
      assert.equal(arrived.different_pixels, 0, `the fleet shows at its destination at once: ${JSON.stringify(arrived)}`);
      assert.ok(gone.different_pixels > gone.pixels_checked / 2, `the origin shows no fleet icon: ${JSON.stringify(gone)}`);
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
      await rightClick(page, { x: Math.round(toLeft) + 17, y: Math.round(toTop) + 5 });
      await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
        null, { timeout: 10_000 });
      const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
      await stableScreen(page, directory, "fleet-icon-en-route-menu");
      // Departed: an en route fleet's moves are grey (FUN_004f9860).
      assert.deepEqual(menu.row_states.slice(0, 2), [["Move", false], ["Confirmed Move", false]], JSON.stringify(menu));
      return { press, delta: [dx, dy], origin, image, arrived, gone, menu, a0 };
    },
  },
  {
    name: "fleet-icon-menu-neutral",
    scenario: QUADRANTS_NEUTRAL_FLEET,
    // The fleet icon's menu for the player's fleet at rest at a neutral
    // system, against A0 2026-11-13-hyperspace-icons/00 (Kubindi, the menu
    // at (232, 144), 172 by 142): Move, Confirmed Move, Planetary
    // Bombardment (its submenu), Planetary Assault grey (no regiment
    // aboard), Encyclopedia, Status grey (an icon's Status), Scrap. The
    // pointer stays at the corner, as A0's arrow does; the arrow's 22 by 40
    // pixels there (with its outline) stay out of the comparison.
    async run(page, faction, setup, directory) {
      const [left, top] = setup.neutral_fleets;
      const at = { x: Math.round(left) + 17, y: Math.round(top) + 5 };
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
      await rightClick(page, at);
      await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
        null, { timeout: 10_000 });
      const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
      assert.deepEqual(menu.row_states, [["Move", true], ["Confirmed Move", true], ["Planetary Bombardment", true],
        ["Planetary Assault", false], ["Encyclopedia", true], ["Status", false], ["Scrap", true]], JSON.stringify(menu));
      assert.deepEqual([menu.left, menu.top, menu.width, menu.height], [at.x, at.y, 172, 142], JSON.stringify(menu));
      await frames(page, 2);
      await page.waitForTimeout(300);
      await frames(page, 2);
      const screen = PNG.sync.read((await shot(page, directory, "fleet-icon-menu")).bytes);
      const a0 = a0Capture("2026-11-13-hyperspace-icons/00-kubindi-fleet-icon-menu-day321.png");
      let compared = null;
      if (a0) {
        compared = menuAgainstA0(screen, [at.x, at.y], a0, [232, 144], { width: 172, height: 142 }, [0, 0, 22, 40]);
        const kinds = ["white", "white", "white", "grey", "white", "grey", "white"];
        assert.deepEqual(compared.a0.map(({ kind }) => kind), kinds, JSON.stringify(compared));
        assert.deepEqual(compared.port.map(({ kind }) => kind), kinds, JSON.stringify(compared));
        // Each row's ink within a pixel of A0's; the right edge within two
        // (the accepted text-width deviation, user decision 2026-10-30).
        assert.ok(compared.inkDiffs.every((d) => d && d.every((value, k) => Math.abs(value) <= (k === 2 ? 2 : 1))),
          `rows' ink against A0: ${JSON.stringify(compared)}`);
        assert.equal(compared.differing, 0, `the frame differs from A0: ${JSON.stringify(compared.first)}`);
      }
      return { menu, compared };
    },
  },
  {
    name: "squadron-neutral-refused",
    scenario: QUADRANTS_NEUTRAL_FLEET,
    // A squadron's Move to a system not of its side is refused with no
    // message (squadrons::check_route, OtherSidePlanet): A0
    // 2026-11-13-icon-drag-move/04, 05 (an X-wing dropped on neutral,
    // unpopulated Kubindi: nothing changes), 08 (its menu Move targeted at
    // Kubindi: nothing changes) and 09 (its Status still attached where it
    // was). Here the player's squadron on the primary planet's Fighter
    // Squadrons page, dragged and then Move-targeted onto the neutral
    // planet: its count, its cell and its Status stay.
    async run(page, faction, setup, directory) {
      const { dat, window } = await openDefenses(page, setup, "primary");
      // The window covers the planets: drag it by its title bar off to
      // the right, as defenses-icon-scrap does.
      const [rowLeft, rowTop] = window.tabs[0][1];
      const grab = { x: Math.round(rowLeft) + 60, y: Math.round(rowTop) - 12 };
      await page.mouse.move(grab.x, grab.y);
      await frames(page);
      await page.mouse.down();
      await frames(page, 2);
      await page.mouse.move(grab.x + 260, grab.y, { steps: 12 });
      await frames(page, 2);
      await page.mouse.up();
      await frames(page, 2);
      await page.waitForTimeout(600);
      const moved = defensesWindow(await latest(page), dat);
      const tab = moved.tabs.find(([name]) => name === "squadrons")[1];
      await click(page, { x: Math.round(tab[0]) + 18, y: Math.round(tab[1]) + 16 });
      const before = defensesWindow(await until(page, "the squadron page opens",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value && w.page === "squadrons"
          && w.cells.length > 0), dat), dat);
      assert.equal(before.counts[2], 1, JSON.stringify(before));
      const cell = before.cells[0];
      const from = { x: Math.round(cell[0] + cell[2] / 2), y: Math.round(cell[1] + cell[3] / 2) };
      const [planet] = setup.neutral;
      const to = { x: Math.round(planet[0] + planet[2] / 2), y: Math.round(planet[1] + planet[3] / 2) };
      const status = async (label) => {
        await page.waitForTimeout(400);
        await page.evaluate(() => {
          window.__openRebellionInterfaceObjectMenu = undefined;
          window.__openRebellionInterfaceStatusRows = undefined;
        });
        await rightClick(page, from);
        await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
          null, { timeout: 10_000 });
        const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
        const height = (menu.height - 2) / menu.rows;
        await click(page, { x: menu.left + menu.width / 2, y: menu.top + 2 + height * (menu.status_window_row + 0.5) });
        await page.waitForFunction(() => window.__openRebellionInterfaceStatusRows?.rows?.length, null, { timeout: 10_000 });
        const rows = await page.evaluate(() => window.__openRebellionInterfaceStatusRows);
        await stableScreen(page, directory, label);
        await page.keyboard.press("Escape");
        await frames(page, 2);
        return { menu, rows };
      };
      const statusBefore = await status("squadron-status-before");
      assert.deepEqual(statusBefore.menu.row_states, [["Move", true], ["Confirmed Move", true], ["Encyclopedia", true],
        ["Status", true], ["Scrap", true]], JSON.stringify(statusBefore.menu));
      assert.equal(statusBefore.rows.rows[0][0], "Attached: ", JSON.stringify(statusBefore.rows));
      // Close the Status window by its Close button (FUN_00443020: 0x65 at
      // its (324, 218)) if Escape left it.
      await page.waitForTimeout(400);
      // The drag: press, rest, walk, rest over the planet, release (A0's).
      await page.mouse.move(from.x, from.y);
      await frames(page);
      await page.mouse.down();
      await frames(page, 2);
      await page.waitForTimeout(300);
      await page.mouse.move(to.x, to.y, { steps: 14 });
      await frames(page, 2);
      await page.waitForTimeout(900);
      await page.mouse.up();
      await frames(page, 2);
      await page.waitForTimeout(3000);
      const dropped = defensesWindow(await latest(page), dat);
      await stableScreen(page, directory, "squadron-dropped-on-neutral");
      assert.deepEqual([dropped.counts[2], dropped.cells.length], [1, 1], JSON.stringify(dropped));
      // The menu's Move, then a click on the neutral planet.
      await page.waitForTimeout(400);
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
      await rightClick(page, from);
      await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
        null, { timeout: 10_000 });
      const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
      assert.equal(menu.move_row, 0, JSON.stringify(menu));
      const height = (menu.height - 2) / menu.rows;
      await click(page, { x: menu.left + menu.width / 2, y: menu.top + 2 + height * (menu.move_row + 0.5) });
      await frames(page, 2);
      await click(page, to);
      await page.waitForTimeout(3000);
      const targeted = defensesWindow(await latest(page), dat);
      await stableScreen(page, directory, "squadron-move-targeted-neutral");
      assert.deepEqual([targeted.counts[2], targeted.cells.length], [1, 1], JSON.stringify(targeted));
      const statusAfter = await status("squadron-status-after");
      assert.deepEqual(statusAfter.rows.rows, statusBefore.rows.rows, JSON.stringify({ statusBefore, statusAfter }));
      return { before: before.counts, dropped: dropped.counts, targeted: targeted.counts,
        status: statusAfter.rows.rows.slice(0, 2), a0: ["04", "05", "08", "09"] };
    },
  },
  {
    name: "missions-icon-drop-refused",
    // CMD-03-C019: the missions icon dropped on another of the player's
    // systems. FUN_00422ce0 (window type 1) puts a kind 0x40 entry into a
    // 0x201 against the drop's +0x70; its team is the members on visible
    // missions (FUN_00512700), each of which the 0x241's validator refuses
    // (FUN_0054d280 -> FUN_00542b60 -> FUN_0054c200 -> FUN_00522b30), so
    // FUN_00487c90 gets the refusal and nothing moves. A0
    // 2026-11-13-defenses-missions-drop/24..30 (the Alliance, Kubindi's
    // missions icon dropped on Jerijador): the drag image is the icon's
    // first bitmap 10777 moved with the pointer, the source drawn selected
    // (10778, 25); released, nothing changes, the icon stays selected (27,
    // 28), no window, no Message Index entry (29, 30). Here the primary's
    // missions icon (the player's agent and the other side's on Diplomacy)
    // is pressed where A0's was, 19 by 11 from its corner, walked by A0's
    // (6, 30), and released on the player's system in uprising: the
    // screen after equals the screen with the icon selected before the
    // drag, the log names the refusal, and the Missions window still lists
    // both missions.
    async run(page, faction, setup, directory, source, consoleLines) {
      const [left, top] = quadrant(setup, "primary", "missions");
      const [dx, dy] = [6, 30];
      const press = { x: Math.round(left) + 19, y: Math.round(top) + 11 };
      // FUN_004a1f60: the other side's mission wins the primary's art.
      const normal = resource(source, art.missions[faction.other]);
      const pressed = resource(source, art.missions[faction.other] + 1);
      let a0 = null;
      const held = a0Capture("2026-11-13-defenses-missions-drop/25-missions-icon-mid-drag.png");
      const released = a0Capture("2026-11-13-defenses-missions-drop/27-released-on-jerijador.png");
      const later = a0Capture("2026-11-13-defenses-missions-drop/28-3s-after-drop.png");
      if (held && released && later && faction.side === 1) {
        // A0's icon is the Alliance's own (Han Solo's Espionage, 23).
        const [a0Normal, a0Pressed] = [resource(source, art.missions[1]), resource(source, art.missions[1] + 1)];
        const pointer = [219 + 19 + dx, 159 + 11 + dy];
        const arrow = (x, y) => x >= pointer[0] && y >= pointer[1] && x < pointer[0] + 12 && y < pointer[1] + 20;
        a0 = {
          held_origin: templateMatch(held, a0Pressed, [200, 240], [140, 180]),
          held_image: templateMatch(held, a0Normal, [200, 250], [170, 210], arrow),
          released: templateMatch(released, a0Pressed, [200, 240], [140, 180]),
          later: templateMatch(later, a0Pressed, [200, 240], [140, 180]),
        };
        assert.deepEqual([a0.held_origin.x, a0.held_origin.y, a0.held_origin.different], [219, 159, 0], JSON.stringify(a0));
        assert.deepEqual([a0.held_image.x - a0.held_origin.x, a0.held_image.y - a0.held_origin.y, a0.held_image.different],
          [dx, dy, 0], JSON.stringify(a0));
        assert.deepEqual([a0.released.x, a0.released.y, a0.released.different], [219, 159, 0], JSON.stringify(a0));
        assert.deepEqual([a0.later.x, a0.later.y, a0.later.different], [219, 159, 0], JSON.stringify(a0));
      }
      // Select the icon first (a press and release in place), for the
      // screen the drop must leave unchanged.
      await click(page, press);
      await page.waitForTimeout(300);
      const selected = await stableScreen(page, directory, "missions-icon-selected");
      const before = compareIcon(selected, [left, top], pressed, "missions-icon-selected", directory);
      assert.equal(before.different_pixels, 0, `the press selects the icon: ${JSON.stringify(before)}`);
      const logged = consoleLines.length;
      await page.mouse.move(press.x, press.y);
      await frames(page);
      await page.mouse.down();
      await frames(page, 2);
      await page.waitForTimeout(300);
      await page.mouse.move(press.x + dx, press.y + dy, { steps: 10 });
      await frames(page, 2);
      await page.waitForTimeout(300);
      await frames(page, 2);
      const screen = PNG.sync.read((await shot(page, directory, "missions-icon-held")).bytes);
      const origin = compareIcon(screen, [left, top], pressed, "missions-icon-origin-pressed", directory);
      const image = compareIcon(screen, [left + dx, top + dy], normal, "missions-icon-drag-image", directory);
      assert.equal(origin.different_pixels, 0, `the origin keeps its pressed art: ${JSON.stringify(origin)}`);
      assert.equal(image.different_pixels, 0, `the drag image: ${JSON.stringify(image)}`);
      const [toLeft, toTop, toWidth, toHeight] = setup.uprising;
      const to = { x: Math.round(toLeft + toWidth / 2), y: Math.round(toTop + toHeight / 2) };
      await page.mouse.move(to.x, to.y, { steps: 12 });
      await frames(page, 2);
      await page.waitForTimeout(900);
      await page.mouse.up();
      await frames(page, 2);
      await page.waitForTimeout(3000);
      const after = await stableScreen(page, directory, "missions-icon-dropped");
      const lines = consoleLines.slice(logged).map(({ text }) => text).filter((text) => text.includes("[interface] command="));
      assert.ok(lines.some((text) => text.includes("command=0x201 missions_icon status=refused")), JSON.stringify(lines));
      assert.ok(!lines.some((text) => text.includes("status=asks")), `no confirmation window: ${JSON.stringify(lines)}`);
      const still = compareIcon(after, [left, top], pressed, "missions-icon-after-drop", directory);
      assert.equal(still.different_pixels, 0, `the icon stays, selected: ${JSON.stringify(still)}`);
      // Nothing else on screen changed: no window, no message.
      let differing = 0;
      for (let i = 0; i < after.data.length; i += 4) {
        if ([0, 1, 2].some((k) => after.data[i + k] !== selected.data[i + k])) differing += 1;
      }
      assert.equal(differing, 0, "the drop changed the screen");
      // The missions go on: the window lists both.
      await doubleClick(page, press);
      const opened = await until(page, "the primary's missions icon opens its window",
        (o, value) => o.missions_windows.some((window) => window.system_dat_id === value), setup.primary_dat_id);
      const window = missionsWindow(opened, setup.primary_dat_id);
      await stableScreen(page, directory, "missions-window-after-drop");
      assert.equal(window.rows.length, 2, JSON.stringify(window));
      assert.ok(window.rows.some(([, side]) => side === faction.side), JSON.stringify(window));
      return { press, delta: [dx, dy], before, origin, image, still, differing, lines, rows: window.rows, a0 };
    },
  },
  {
    name: "defenses-icon-drop-a0",
    scenario: QUADRANTS_FREE_TEAM,
    // CMD-03-C019 and CMD-03-C018: the defenses icon of a free team
    // dropped on another of the player's systems. FUN_00422ce0 (window
    // type 1) puts the kind 8 entry into a 0x201 against +0x70; its team is
    // the system's regiments, squadrons, characters and special forces of
    // the side (FUN_00512700 kind 8), and FUN_00487cc0 asks nothing for an
    // icon's Move (its first id is the system, whose container is a sector;
    // ghidra/notes/sector-icon-menus.md). A0
    // 2026-11-13-defenses-missions-drop (the Alliance, Jerijador's icon at
    // (220, 233) pressed at (225, 246)): its menu before, Move, Confirmed
    // Move, Encyclopedia white, Status grey, Scrap white (00, the menu at
    // (225, 246)); the drag image 10773 moved with the pointer, the source
    // 10774 (03: by (75, -6)); released on Yavin, no confirmation, no
    // message, the icon stays and is drawn unselected (05: FUN_0045d140's
    // refresh deselects on the team's change notices); its menu after,
    // Move and Confirmed Move grey (the team is gone), Encyclopedia white,
    // Status grey, Scrap white (21, the menu at (222, 245): Scrap's team
    // keeps the facilities). Here the primary's team is the player's free
    // agent and regiment, with the battery behind; the drop is on the
    // player's system in uprising.
    async run(page, faction, setup, directory, source, consoleLines) {
      const [left, top] = quadrant(setup, "primary", "defenses");
      const [dx, dy] = [75, -6];
      const press = { x: Math.round(left) + 5, y: Math.round(top) + 13 };
      const normal = resource(source, art.defenses[faction.side]);
      const pressed = resource(source, art.defenses[faction.side] + 1);
      const result = { press, delta: [dx, dy] };
      const a0Before = a0Capture("2026-11-13-defenses-missions-drop/00-jerijador-defenses-icon-menu.png");
      const a0Held = a0Capture("2026-11-13-defenses-missions-drop/03-defenses-icon-mid-drag.png");
      const a0Dropped = a0Capture("2026-11-13-defenses-missions-drop/05-released-on-yavin.png");
      const a0After = a0Capture("2026-11-13-defenses-missions-drop/21-jerijador-defenses-icon-menu-after.png");
      const alliance = faction.side === 1 && a0Before && a0Held && a0Dropped && a0After;
      if (alliance) {
        const pointer = [220 + 5 + dx, 233 + 13 + dy];
        const arrow = (x, y) => x >= pointer[0] && y >= pointer[1] && x < pointer[0] + 12 && y < pointer[1] + 20;
        result.a0 = {
          held_origin: templateMatch(a0Held, pressed, [200, 240], [220, 250]),
          held_image: templateMatch(a0Held, normal, [270, 320], [210, 240], arrow),
          dropped: templateMatch(a0Dropped, normal, [200, 240], [220, 250]),
        };
        assert.deepEqual([result.a0.held_origin.x, result.a0.held_origin.y, result.a0.held_origin.different],
          [220, 233, 0], JSON.stringify(result.a0));
        assert.deepEqual([result.a0.held_image.x - 220, result.a0.held_image.y - 233, result.a0.held_image.different],
          [dx, dy, 0], JSON.stringify(result.a0));
        assert.deepEqual([result.a0.dropped.x, result.a0.dropped.y, result.a0.dropped.different],
          [220, 233, 0], JSON.stringify(result.a0));
      }
      const openMenu = async (label) => {
        await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
        await rightClick(page, press);
        await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
          null, { timeout: 10_000 });
        const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
        assert.deepEqual([menu.left, menu.top, menu.width, menu.height], [press.x, press.y, 104, 102], JSON.stringify(menu));
        // The pointer stays at the corner, as A0's arrow does.
        await frames(page, 2);
        await page.waitForTimeout(300);
        await frames(page, 2);
        const screen = PNG.sync.read((await shot(page, directory, label)).bytes);
        return { menu, screen };
      };
      const compareMenu = (screen, a0, corner, kinds) => {
        const compared = menuAgainstA0(screen, [press.x, press.y], a0, corner, { width: 104, height: 102 }, [0, 0, 22, 40]);
        assert.deepEqual(compared.a0.map(({ kind }) => kind), kinds, JSON.stringify(compared));
        assert.deepEqual(compared.port.map(({ kind }) => kind), kinds, JSON.stringify(compared));
        // Each row's ink within a pixel of A0's; the right edge within two
        // (the accepted text-width deviation, user decision 2026-10-30).
        assert.ok(compared.inkDiffs.every((d) => d && d.every((value, k) => Math.abs(value) <= (k === 2 ? 2 : 1))),
          `rows' ink against A0: ${JSON.stringify(compared)}`);
        assert.equal(compared.differing, 0, `the frame differs from A0: ${JSON.stringify(compared.first)}`);
        return compared;
      };
      // Before: the team at rest, both moves enabled (A0 00).
      const first = await openMenu("defenses-icon-menu-before");
      assert.deepEqual(first.menu.row_states,
        [["Move", true], ["Confirmed Move", true], ["Encyclopedia", true], ["Status", false], ["Scrap", true]],
        JSON.stringify(first.menu));
      result.before = { menu: first.menu };
      if (alliance) {
        result.before.a0 = compareMenu(first.screen, a0Before, [225, 246], ["white", "white", "white", "grey", "white"]);
      }
      await page.keyboard.press("Escape");
      await frames(page, 2);
      await page.waitForTimeout(300);
      const logged = consoleLines.length;
      await page.mouse.move(press.x, press.y);
      await frames(page);
      await page.mouse.down();
      await frames(page, 2);
      await page.waitForTimeout(300);
      await page.mouse.move(press.x + dx, press.y + dy, { steps: 10 });
      await frames(page, 2);
      await page.waitForTimeout(300);
      await frames(page, 2);
      const held = PNG.sync.read((await shot(page, directory, "defenses-icon-held")).bytes);
      result.origin = compareIcon(held, [left, top], pressed, "defenses-icon-origin-pressed", directory);
      result.image = compareIcon(held, [left + dx, top + dy], normal, "defenses-icon-drag-image", directory);
      assert.equal(result.origin.different_pixels, 0, `the origin keeps its pressed art: ${JSON.stringify(result.origin)}`);
      assert.equal(result.image.different_pixels, 0, `the drag image: ${JSON.stringify(result.image)}`);
      const [toLeft, toTop, toWidth, toHeight] = setup.uprising;
      const to = { x: Math.round(toLeft + toWidth / 2), y: Math.round(toTop + toHeight / 2) };
      await page.mouse.move(to.x, to.y, { steps: 12 });
      await frames(page, 2);
      await page.waitForTimeout(900);
      await page.mouse.up();
      await frames(page, 2);
      await page.waitForTimeout(3000);
      const dropped = await stableScreen(page, directory, "defenses-icon-dropped");
      result.lines = consoleLines.slice(logged).map(({ text }) => text).filter((text) => text.includes("[interface] command="));
      assert.ok(!result.lines.some((text) => text.includes("status=asks") || text.includes("status=refused")),
        `no confirmation window, no refusal: ${JSON.stringify(result.lines)}`);
      // The battery keeps the icon, drawn unselected (A0 05).
      result.stays = compareIcon(dropped, [left, top], normal, "defenses-icon-after-drop", directory);
      assert.equal(result.stays.different_pixels, 0, `the icon stays, unselected: ${JSON.stringify(result.stays)}`);
      // After: the team gone, both moves grey (A0 21).
      const second = await openMenu("defenses-icon-menu-after");
      assert.deepEqual(second.menu.row_states,
        [["Move", false], ["Confirmed Move", false], ["Encyclopedia", true], ["Status", false], ["Scrap", true]],
        JSON.stringify(second.menu));
      result.after = { menu: second.menu };
      if (alliance) {
        result.after.a0 = compareMenu(second.screen, a0After, [222, 245], ["grey", "grey", "white", "grey", "white"]);
      }
      await page.keyboard.press("Escape");
      await frames(page, 2);
      await page.waitForTimeout(300);
      // The agent's part is one Move mission (FUN_0053c810), dispatched;
      // its members depart in the mission's phase 4 (FUN_00556430,
      // FUN_00556390), which the paused fixture does not reach, so the
      // agent is still listed here. A0 07..11 (day 355, two days on) show
      // Jerijador's lists empty and its shields kept.
      assert.ok(consoleLines.slice(logged).some(({ text }) => /members_move members=1 .*result=Ok/.test(text)),
        JSON.stringify(consoleLines.slice(logged).map(({ text }) => text).filter((text) => text.includes("[interface]"))));
      // The Defenses window: the regiment gone, the agent on its Move, the
      // battery there.
      const { window } = await openDefenses(page, setup, "primary");
      await stableScreen(page, directory, "defenses-window-after-drop");
      assert.deepEqual(window.counts, [1, 0, 0, 0, 1], JSON.stringify(window));
      result.counts = window.counts;
      return result;
    },
  },
  {
    name: "two-defenses",
    // Two systems' Defenses windows stay open together, and minimizing one
    // sends only it to the rail (A0 personnel-states/25: Pzob and Mon
    // Calamari open; Pzob minimized to the rail, Mon Calamari still open).
    async run(page, faction, setup, directory) {
      const first = await openDefenses(page, setup, "primary");
      // It opens over the sector window; its title strip moves it right
      // (FUN_00606ee0), clear of the second system's icon.
      const [fx, fy] = corner(first.window);
      await page.mouse.move(fx + 120, fy + 9);
      await frames(page);
      await page.mouse.down();
      await frames(page);
      await page.mouse.move(fx + 120 + 260, fy + 9, { steps: 10 });
      await frames(page);
      await page.mouse.up();
      await frames(page, 3);
      await page.mouse.move(2, 2);
      await page.waitForTimeout(500);
      const second = await openDefenses(page, setup, "second");
      const both = await until(page, "both Defenses windows are open",
        (o, value) => value.every((dat) => o.defenses_windows.some((w) => w.system_dat_id === dat)),
        [first.dat, second.dat]);
      await stableScreen(page, directory, "two-open");
      // A window over another takes the click on its covered control:
      // A0 personnel-states/36 (Pzob's Fleet window over Xyquine's tabs)
      // leaves Xyquine on its page. The second window's title strip is
      // dragged onto one of the first's other tabs, which is then clicked.
      const firstWindow = defensesWindow(both, first.dat);
      const [, tab] = firstWindow.tabs.find(([name]) => name !== firstWindow.page);
      const covered = { x: Math.round(tab[0] + tab[2] / 2), y: Math.round(tab[1] + tab[3] / 2) };
      const [sx, sy] = corner(defensesWindow(both, second.dat));
      await page.mouse.move(sx + 120, sy + 9);
      await frames(page);
      await page.mouse.down();
      await frames(page);
      await page.mouse.move(covered.x, covered.y, { steps: 10 });
      await frames(page);
      await page.mouse.up();
      await frames(page, 3);
      const moved = await until(page, "the second window covers the first's tab",
        (o, value) => {
          const w = o.defenses_windows.find((d) => d.system_dat_id === value.dat);
          if (!w) return false;
          const [x, y] = [w.tabs.find(([name]) => name === "personnel")[1][0] - 28, w.tabs.find(([name]) => name === "personnel")[1][1] - 20];
          return value.at.x >= x && value.at.x < x + 235 && value.at.y >= y && value.at.y < y + 300;
        }, { dat: second.dat, at: covered });
      await page.mouse.move(2, 2);
      await page.waitForTimeout(500);
      await click(page, covered);
      await page.mouse.move(2, 2);
      await frames(page, 3);
      await page.waitForTimeout(300);
      const after = await until(page, "both windows stay open",
        (o, value) => value.every((dat) => o.defenses_windows.some((w) => w.system_dat_id === dat)),
        [first.dat, second.dat]);
      assert.equal(defensesWindow(after, first.dat).page, firstWindow.page,
        `a covered tab took the click: ${JSON.stringify({ covered, first: defensesWindow(after, first.dat) })}`);
      await stableScreen(page, directory, "covered-click");
      const top = defensesWindow(after, second.dat);
      // The click lands on the window above: its own tab there takes it.
      assert.notEqual(top.page, defensesWindow(both, second.dat).page,
        `the top window took no click: ${JSON.stringify({ covered, top })}`);
      const origin = corner(defensesWindow(after, second.dat));
      await click(page, { x: origin[0] + 210, y: origin[1] + 9 });
      const railed = await until(page, "only the second window goes to the rail",
        (o, value) => !o.defenses_windows.some((w) => w.system_dat_id === value[1])
          && o.defenses_windows.some((w) => w.system_dat_id === value[0])
          && o.rail.some(([system, kind]) => system === value[1] && kind === "defenses")
          && !o.rail.some(([system]) => system === value[0]), [first.dat, second.dat]);
      await stableScreen(page, directory, "one-railed");
      return { open: both.defenses_windows.length, covered, moved: corner(defensesWindow(moved, second.dat)), topPage: top.page, rail: railed.rail };
    },
  },
  {
    name: "defenses-other-side",
    // +0x148 is the system's side: the other side's system lists only its
    // objects, of which the fixture leaves none, so every tab shows that
    // side's empty art and the player's regiment there is not listed.
    async run(page, faction, setup, directory, source) {
      const { at, window } = await openDefenses(page, setup, "second");
      assert.equal(window.side, faction.other, JSON.stringify(window));
      assert.deepEqual(window.counts, [0, 0, 0, 0, 0]);
      assert.equal(window.garrison, null);
      const screenshot = await stableScreen(page, directory, "other-side");
      const checks = compareTabs(screenshot, source, window, directory, "other-side");
      assertExact(checks);
      // A0 coruscant-676/14-after-movie (the VM's Empire game, day 679):
      // after the Alliance seized Coruscant, the Empire's player sees its
      // Defenses window with side 1's focused title strip 10299 (FUN_004a9800
      // by the system's side) at (202, 93), the window at (200, 91). The
      // strip, its sector, minimize and close buttons match the port's
      // pixel for pixel outside the name's label (x 19..198, the window's
      // own); the name's ink, black, starts within a pixel of A0's. The
      // three buttons' bottom-left key pixels are compared too: each shows
      // the window surface +0xa4, 10577's black at (3, 16), (204, 16) and
      // (218, 16), whatever the strip's side or focus (FUN_00606c90 passes
      // FUN_006073d0's +0xa4; FUN_00602d30 -> FUN_005fd0f0; the strip goes
      // to the DC only, FUN_004a9800). The green in A0 fleet-aboard/00..02
      // and status/22 is the Fleet window's plain blit (FUN_004a4b10), and
      // the red in menus2/11 is the Create Mission dialog's, not this window.
      let a0 = null;
      const a0File = path.join(root, ".artifacts/a0-vm/coruscant-676/14-after-movie.png");
      if (faction.side === 2 && fs.existsSync(a0File)) {
        const shot = PNG.sync.read(fs.readFileSync(a0File));
        const [ox, oy] = window.origin;
        const pixel = (png, x0, y0, x, y) =>
          Array.from(png.data.subarray(((y0 + y) * png.width + x0 + x) * 4, ((y0 + y) * png.width + x0 + x) * 4 + 3));
        let differing = 0;
        let checked = 0;
        const label = { a0: null, port: null };
        const grow = (box, x, y) => box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
        for (let y = 2; y < 18; y += 1) for (let x = 2; x < 233; x += 1) {
          const [a, p] = [pixel(shot, 200, 91, x, y), pixel(screenshot, ox, oy, x, y)];
          if (x >= 19 && x < 199) {
            if (a.every((value) => value < 64)) label.a0 = grow(label.a0, x, y);
            if (p.every((value) => value < 64)) label.port = grow(label.port, x, y);
            continue;
          }
          checked += 1;
          if (a.some((value, k) => value !== p[k])) differing += 1;
        }
        a0 = { checked, differing, label };
        assert.equal(differing, 0, `the title strip against A0: ${JSON.stringify(a0)}`);
        assert.ok(label.a0 && label.port && Math.abs(label.a0[0] - label.port[0]) <= 1
          && Math.abs(label.a0[1] - label.port[1]) <= 1 && Math.abs(label.a0[3] - label.port[3]) <= 1,
        `the name's ink against A0: ${JSON.stringify(a0)}`);
      }
      return { at, window, checks, a0 };
    },
  },
  {
    name: "coruscant-seized",
    scenario: CORUSCANT_SEIZED,
    factions: ["empire"],
    // A0 coruscant-676/14-after-movie (the VM's Empire game, day 679): after
    // Get Immediate Result gave the Alliance Coruscant, the Empire's player
    // opened Coruscant's Defenses window (FUN_004a8790) at (200, 91). It
    // shows the system's side (+0x148, the Alliance): the red strip 10299
    // (FUN_004a9800), Personnel pressed over an empty list, Trooper
    // Regiments full, the other tabs empty (FUN_004a9ce0). The fixture
    // leaves the Alliance one regiment there, in the Empire's sight.
    async run(page, faction, setup, directory, source) {
      const { at, window } = await openDefenses(page, setup, "second");
      assert.equal(window.side, 1, JSON.stringify(window));
      assert.equal(window.page, "personnel", JSON.stringify(window));
      assert.deepEqual(window.counts, [0, 1, 0, 0, 0], JSON.stringify(window));
      assert.deepEqual(window.rows, [], JSON.stringify(window));
      assert.equal(window.garrison, null);
      const screenshot = await stableScreen(page, directory, "coruscant");
      const checks = compareTabs(screenshot, source, window, directory, "coruscant");
      assertExact(checks);
      const a0File = path.join(root, ".artifacts/a0-vm/coruscant-676/14-after-movie.png");
      if (!fs.existsSync(a0File)) return { at, window, checks, a0: "skipped: A0 file missing" };
      // The whole 235 by 304 window against A0's, every pixel but the text
      // (the title's name from (19, 2), FUN_004a9800; the page name
      // "Personnel" centred under the tabs) and A0's mouse pointer, which
      // stood over the empty list (all 179 other differing pixels lie in
      // x 120..141, y 159..180). The list is empty in both, so its
      // backdrop (10577) is compared.
      const whole = wholeWindow(screenshot, window.origin, a0File, [200, 91], [
        [19, 2, 180, 16], [2, 54, 231, 13], [118, 157, 26, 26],
      ]);
      assert.ok(whole.compared > 64_000 && whole.differing === 0,
        `the whole Defenses window of Coruscant against A0: ${JSON.stringify(whole)}`);
      // The name's ink, black on the strip: A0 and the port both name
      // Coruscant. Its box's left, top and bottom within a pixel of A0's,
      // its width within 15% (the accepted text-width deviation).
      const shot = PNG.sync.read(fs.readFileSync(a0File));
      const ink = (png, [ox, oy]) => {
        let box = null;
        for (let y = 2; y < 18; y += 1) for (let x = 19; x < 199; x += 1) {
          const i = ((oy + y) * png.width + ox + x) * 4;
          if (![0, 1, 2].every((k) => png.data[i + k] < 64)) continue;
          box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
        }
        return box;
      };
      const name = { a0: ink(shot, [200, 91]), port: ink(screenshot, window.origin) };
      const ratio = name.a0 && name.port ? (name.port[2] - name.port[0] + 1) / (name.a0[2] - name.a0[0] + 1) : 0;
      assert.ok(name.a0 && name.port && [0, 1, 3].every((k) => Math.abs(name.a0[k] - name.port[k]) <= 1)
        && ratio > 0.85 && ratio < 1.15, `Coruscant's name against A0: ${JSON.stringify({ ...name, ratio })}`);
      // The page name's white ink in the same way.
      const white = (png, [ox, oy]) => {
        let box = null;
        for (let y = 54; y < 67; y += 1) for (let x = 2; x < 233; x += 1) {
          const i = ((oy + y) * png.width + ox + x) * 4;
          if (![0, 1, 2].every((k) => png.data[i + k] > 200)) continue;
          box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
        }
        return box;
      };
      const label = { a0: white(shot, [200, 91]), port: white(screenshot, window.origin) };
      const labelRatio = label.a0 && label.port ? (label.port[2] - label.port[0] + 1) / (label.a0[2] - label.a0[0] + 1) : 0;
      assert.ok(label.a0 && label.port && [0, 1, 3].every((k) => Math.abs(label.a0[k] - label.port[k]) <= 1)
        && labelRatio > 0.85 && labelRatio < 1.15, `the page name against A0: ${JSON.stringify({ ...label, labelRatio })}`);
      return { at, window, checks, a0: { whole, name: { ...name, ratio }, label: { ...label, ratio: labelRatio } } };
    },
  },
  {
    name: "defenses-multi-select",
    scenario: CORUSCANT_SQUADRONS,
    factions: ["empire"],
    // The Defenses window's list (flags 0xa0000, FUN_004a8790) is no
    // single-selection list: Ctrl adds a squadron, and Shift adds every
    // cell meeting the rectangle from the last press to this one in the
    // list's content pixels (FUN_006083c0, FUN_00609590), across a scroll.
    async run(page, faction, setup, directory) {
      const { dat, window } = await openDefenses(page, setup, "second");
      const tab = window.tabs.find(([name]) => name === "squadrons")[1];
      await page.mouse.move(2, 2);
      await page.waitForTimeout(300);
      await click(page, { x: tab[0] + 18, y: tab[1] + 16 });
      const opened = defensesWindow(await until(page, "the squadrons page opens",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value && w.page === "squadrons"), dat), dat);
      const origin = corner(opened);
      const cell = (w, index) => ({ x: w.cells[index][0] + 30, y: w.cells[index][1] + 12 });
      const withKey = async (key, at) => {
        await page.keyboard.down(key);
        await frames(page);
        await click(page, at);
        await page.waitForTimeout(300);
        await page.keyboard.up(key);
        await frames(page);
      };
      await click(page, cell(opened, 0));
      await page.waitForTimeout(300);
      await withKey("Control", cell(opened, 1));
      const two = defensesWindow(await until(page, "Ctrl+click selects two squadrons",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value && JSON.stringify(w.selection) === "[0,1]"), dat), dat);
      await shot(page, directory, "two-selected");
      // Page the list to its end on the bar's track, then Shift+click the
      // tenth squadron (column 0): columns 0..1 of every line join.
      await page.mouse.move(2, 2);
      await page.waitForTimeout(300);
      await click(page, { x: origin[0] + 224, y: origin[1] + 260 });
      const end = defensesWindow(await until(page, "the track pages the list to its end",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value[0] && w.cells[9][1] === value[1]),
        [dat, origin[1] + 81 + 210 - 75]), dat);
      await page.mouse.move(2, 2);
      await page.waitForTimeout(300);
      await withKey("Shift", cell(end, 9));
      const range = defensesWindow(await until(page, "Shift+click adds the range across the scroll",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value && JSON.stringify(w.selection) === "[0,1,3,4,6,7,9]"), dat), dat);
      await shot(page, directory, "shift-range");
      return { two, range };
    },
  },
  {
    name: "defenses-a0-regiments",
    scenario: CORUSCANT_REGIMENTS,
    factions: ["alliance"],
    // A0 2026-11-13-drag/31..33 (Averam's Defenses window at (82, 86), the
    // Alliance): a click on the first regiment and a Ctrl+click on the
    // second frame each image with the side's 10578 (a 61 by 25 red outline
    // at the cell's corner) and turn its name red (FUN_004a9ab0); no frame
    // goes round the name. Held and moved, the drag image is the two framed
    // images on the list's 70-pixel pitch without names (FUN_00609960), at
    // the press's offset, drawn over the window's edge (32). The fixture's
    // Coruscant holds the player's two Alliance Army Regiments.
    async run(page, faction, setup, directory) {
      const a0File = (name) => path.join(root, ".artifacts/a0-vm/2026-11-13-drag", name);
      const a0Origin = [82, 86];
      const { dat, window } = await openDefenses(page, setup, "second");
      const tab = window.tabs.find(([name]) => name === "regiments")[1];
      await page.mouse.move(2, 2);
      await page.waitForTimeout(300);
      await click(page, { x: tab[0] + 18, y: tab[1] + 16 });
      const opened = defensesWindow(await until(page, "the regiments page opens",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value && w.page === "regiments"), dat), dat);
      assert.equal(opened.rows.length, 2, JSON.stringify(opened));
      const origin = corner(opened);
      // A0 31's click points (119, 180) and (189, 180): the cell's (30, 13).
      const cell = (index) => ({ x: origin[0] + 7 + 70 * index + 30, y: origin[1] + 81 + 13 });
      const withKey = async (key, at) => {
        await page.keyboard.down(key);
        await frames(page);
        await click(page, at);
        await page.waitForTimeout(300);
        await page.keyboard.up(key);
        await frames(page);
      };
      await click(page, cell(0));
      await page.waitForTimeout(300);
      await withKey("Control", cell(1));
      await until(page, "Ctrl+click selects both regiments",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value && JSON.stringify(w.selection) === "[0,1]"), dat);
      const two = await stableScreen(page, directory, "two-selected");
      const rgbAt = (png, x, y) => {
        const i = (y * png.width + x) * 4;
        return [png.data[i], png.data[i + 1], png.data[i + 2]];
      };
      const isRed = ([r, g, b]) => r === 255 && g === 0 && b === 0;
      const isWhite = ([r, g, b]) => r === 255 && g === 255 && b === 255;
      // The red pixels of a window-relative box [x0, y0, x1, y1], as
      // window coordinates.
      const red = (png, [wx, wy], [x0, y0, x1, y1]) => {
        const set = new Set();
        for (let y = y0; y <= y1; y += 1) for (let x = x0; x <= x1; x += 1) {
          if (isRed(rgbAt(png, wx + x, wy + y))) set.add(`${x},${y}`);
        }
        return set;
      };
      const same = (a, b, label) => {
        const onlyPort = [...a].filter((key) => !b.has(key));
        const onlyA0 = [...b].filter((key) => !a.has(key));
        assert.ok(onlyPort.length === 0 && onlyA0.length === 0,
          `${label}: ${JSON.stringify({ port: a.size, a0: b.size, onlyPort: onlyPort.slice(0, 12), onlyA0: onlyA0.slice(0, 12) })}`);
        return { pixels: a.size };
      };
      const checks = {};
      // The images' band (the cells' y 0..25): the two 10578 outlines.
      const band = [7, 81, 228, 107];
      const frame = (x) => {
        const set = new Set();
        for (let k = 0; k < 61; k += 1) {
          set.add(`${x + k},81`);
          set.add(`${x + k},105`);
        }
        for (let k = 0; k < 25; k += 1) {
          set.add(`${x},${81 + k}`);
          set.add(`${x + 60},${81 + k}`);
        }
        // 10578's bottom-left pixel is its blue key.
        set.delete(`${x},105`);
        return set;
      };
      checks.frames = same(red(two, origin, band), new Set([...frame(7), ...frame(77)]), "the two 10578 frames");
      // The names: red ink, no white, no frame (the cells' y 27..70). The
      // port's text is smoothed, so its ink is any strongly red pixel, and
      // its box is compared (the accepted text-width deviation).
      const names = [7, 108, 228, 150];
      const reddish = ([r, g, b]) => r >= 128 && g < 64 && b < 96;
      const ink = (png, [wx, wy]) => {
        let reds = 0;
        let whites = 0;
        let box = null;
        for (let y = names[1]; y <= names[3]; y += 1) for (let x = names[0]; x <= names[2]; x += 1) {
          const value = rgbAt(png, wx + x, wy + y);
          if (isWhite(value)) whites += 1;
          if (!reddish(value)) continue;
          reds += 1;
          box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
        }
        return { reds, whites, box };
      };
      checks.names = ink(two, origin);
      assert.ok(checks.names.reds > 100 && checks.names.whites === 0, `the selected names: ${JSON.stringify(checks.names)}`);
      if (fs.existsSync(a0File("31-averam-two-regiments-ctrl-selected-red-names.png"))) {
        const a0 = PNG.sync.read(fs.readFileSync(a0File("31-averam-two-regiments-ctrl-selected-red-names.png")));
        checks.frames_a0 = same(red(two, origin, band), red(a0, a0Origin, band), "the frames against A0 31");
        checks.names_a0 = { port: checks.names, a0: ink(a0, a0Origin) };
        const [p, q] = [checks.names.box, checks.names_a0.a0.box];
        const ratio = (p[2] - p[0] + 1) / (q[2] - q[0] + 1);
        // The accepted text deviation: "Alliance Army Regiment" in A0's
        // GDI font against the port's, both red, the ink box's left and top
        // within 2 pixels and its width within 15%.
        assert.ok(checks.names_a0.a0.whites === 0 && Math.abs(p[0] - q[0]) <= 2 && Math.abs(p[1] - q[1]) <= 2
          && ratio > 0.85 && ratio < 1.15,
          `the red names against A0 31: ${JSON.stringify({ ...checks.names_a0, ratio })}`);
        // The list's every pixel but the names (text), on the same
        // backdrop 10577.
        checks.list_a0 = wholeWindow(two, origin, a0File("31-averam-two-regiments-ctrl-selected-red-names.png"),
          a0Origin, [[0, 0, 235, 81], [0, 108, 235, 44], [0, 291, 235, 13]]);
        assert.equal(checks.list_a0.differing, 0, `the list against A0 31: ${JSON.stringify(checks.list_a0)}`);
      }

      // A0 32: pressed at (119, 180) and held at (280, 220), +(161, 40).
      const start = cell(0);
      await page.mouse.move(start.x, start.y);
      await frames(page);
      await page.mouse.down();
      await frames(page);
      await page.mouse.move(start.x + 80, start.y + 20, { steps: 4 });
      await page.mouse.move(start.x + 161, start.y + 40, { steps: 4 });
      await page.waitForTimeout(150);
      await frames(page, 2);
      const held = PNG.sync.read((await shot(page, directory, "drag-held")).bytes);
      const image = [7 + 161, 81 + 40, 7 + 161 + 130, 81 + 40 + 24];
      checks.drag_frames = same(
        new Set([...red(held, origin, [image[0], image[1], image[0] + 60, image[3]]),
          ...red(held, origin, [image[0] + 70, image[1], image[2], image[3]])]),
        new Set([...frame(7), ...frame(77)].map((key) => {
          const [x, y] = key.split(",").map(Number);
          return `${x + 161},${y + 40}`;
        })),
        "the drag image's frames at the press offset");
      if (fs.existsSync(a0File("32-two-regiments-mid-drag-inside-window.png"))) {
        const a0 = PNG.sync.read(fs.readFileSync(a0File("32-two-regiments-mid-drag-inside-window.png")));
        const a0Selected = PNG.sync.read(fs.readFileSync(a0File("31-averam-two-regiments-ctrl-selected-red-names.png")));
        // The image's pixels: those of A0 32's two boxes that copy A0 31's
        // cells at the press offset and differ from what A0 31 (the same
        // screen without the image) shows there; the port must show the
        // same.
        let art = 0;
        const wrong = [];
        for (const left of [image[0], image[0] + 70]) {
          for (let y = image[1]; y <= image[3]; y += 1) for (let x = left; x <= left + 60; x += 1) {
            const shown = rgbAt(a0, a0Origin[0] + x, a0Origin[1] + y);
            const source = rgbAt(a0Selected, a0Origin[0] + x - 161, a0Origin[1] + y - 40);
            const under = rgbAt(a0Selected, a0Origin[0] + x, a0Origin[1] + y);
            if (![0, 1, 2].every((k) => shown[k] === source[k])
              || [0, 1, 2].every((k) => shown[k] === under[k])) continue;
            art += 1;
            const port = rgbAt(held, origin[0] + x, origin[1] + y);
            if (![0, 1, 2].every((k) => port[k] === shown[k])) wrong.push([x, y]);
          }
        }
        checks.drag_a0 = { art, differing: wrong.length, first: wrong.slice(0, 12) };
        assert.ok(art > 300 && wrong.length === 0, `the drag image against A0 32: ${JSON.stringify(checks.drag_a0)}`);
      }
      // Back on the press point: no drop, and the held click keeps the
      // first regiment alone (FUN_006083c0 case 0x202).
      await page.mouse.move(start.x, start.y, { steps: 6 });
      await frames(page);
      await page.mouse.up();
      await page.waitForTimeout(300);
      const released = defensesWindow(await until(page, "the release keeps the first regiment alone",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value && JSON.stringify(w.selection) === "[0]"), dat), dat);
      assert.equal(released.rows.length, 2, "nothing moved");
      return checks;
    },
  },
  {
    name: "defenses-list-bar",
    scenario: CORUSCANT_SQUADRONS,
    factions: ["empire"],
    // A0 2026-11-11-icon-menu-topic/05 and 06 (the VM's Empire game):
    // Coruscant's Defenses window at (203, 91) on its Fighter Squadrons
    // page, ten TIE Fighter squadrons in four lines of three, past the
    // list's 210 pixels. FUN_004a8790 gives the list the bar of RCDATA 10700
    // (FUN_0060a490 with 0x29cc) at its right edge, window x 219..228 from
    // y 81: 05 shows it from the top, 06 at the end (280 + 5 - 210 = 75,
    // FUN_00609ae0), the last line's squadron selected. The port's bar
    // against both, every pixel; the scrolled squadrons' minis against 06.
    async run(page, faction, setup, directory, source) {
      const { dat, window } = await openDefenses(page, setup, "second");
      assert.equal(window.side, 2, JSON.stringify(window));
      assert.equal(window.counts[2], 10, JSON.stringify(window));
      const origin = corner(window);
      const tab = window.tabs.find(([name]) => name === "squadrons")[1];
      await page.mouse.move(2, 2);
      await page.waitForTimeout(300);
      await click(page, { x: tab[0] + 18, y: tab[1] + 16 });
      const squadrons = defensesWindow(await until(page, "the squadrons tab opens its page",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value && w.page === "squadrons"), dat), dat);
      assert.equal(squadrons.rows.length, 10, JSON.stringify(squadrons));
      assert.equal(squadrons.cells.length, 10, JSON.stringify(squadrons));
      assert.equal(squadrons.cells[9][1], origin[1] + 81 + 210, JSON.stringify(squadrons.cells));
      const a0Dir = path.join(root, ".artifacts/a0-vm/2026-11-11-icon-menu-topic");
      const a0Top = path.join(a0Dir, "05-tie-menu-move-white.png");
      const a0End = path.join(a0Dir, "06-last-tie-menu-move-white.png");
      const a0Origin = [203, 91];
      // Each region's pixels, window-relative, against A0's.
      const regionDiff = (port, a0File, regions) => {
        const a0 = PNG.sync.read(fs.readFileSync(a0File));
        const rgb = (png, x, y) => {
          const i = (y * png.width + x) * 4;
          return (png.data[i] << 16) | (png.data[i + 1] << 8) | png.data[i + 2];
        };
        let compared = 0;
        const differing = [];
        for (const [rx, ry, rw, rh] of regions) {
          for (let y = ry; y < ry + rh; y += 1) {
            for (let x = rx; x < rx + rw; x += 1) {
              compared += 1;
              if (rgb(port, origin[0] + x, origin[1] + y) !== rgb(a0, a0Origin[0] + x, a0Origin[1] + y)) {
                differing.push([x, y]);
              }
            }
          }
        }
        return { a0: path.basename(a0File), compared, differing: differing.length, first: differing.slice(0, 12) };
      };
      const bar = [219, 81, 10, 210];
      let screenshot = await stableScreen(page, directory, "top");
      const top = regionDiff(screenshot, a0Top, [bar]);
      assert.ok(top.compared === 2100 && top.differing === 0, `the bar from the top against A0 05: ${JSON.stringify(top)}`);

      // The down arrow steps the list by 10 (FUN_004a8790's step).
      await click(page, { x: origin[0] + 224, y: origin[1] + 286 });
      const stepped = defensesWindow(await until(page, "the down arrow scrolls the list by its step",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value[0] && w.cells[0][1] === value[1]),
        [dat, origin[1] + 81 - 10]), dat);
      // A press on the track below the thumb pages by the list's height,
      // clamped to the end (FUN_0060fd50).
      await page.mouse.move(2, 2);
      await page.waitForTimeout(300);
      await click(page, { x: origin[0] + 224, y: origin[1] + 260 });
      const end = defensesWindow(await until(page, "the track pages the list to its end",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value[0] && w.cells[9][1] === value[1]),
        [dat, origin[1] + 81 + 210 - 75]), dat);
      const last = end.cells[9];
      await page.mouse.move(2, 2);
      await page.waitForTimeout(300);
      await click(page, { x: last[0] + 30, y: last[1] + 12 });
      const selected = defensesWindow(await until(page, "a click selects the tenth squadron",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value && w.selected === 9), dat), dat);
      assert.equal(selected.cells[9][1], last[1], "the selection keeps the list's place");
      screenshot = await stableScreen(page, directory, "end");
      // A0 06's minis of the second and third lines, above their names
      // (TIE Fighter, FUN_004a9ab0; the text keeps its accepted width
      // deviation): the second line cut by the list's top at 81 - 5.
      const minis = [];
      for (const column of [0, 1, 2]) {
        minis.push([7 + 70 * column, 81, 70, 76 + 27 - 81]);
        minis.push([7 + 70 * column, 146, 70, 27]);
      }
      const atEnd = regionDiff(screenshot, a0End, [bar]);
      const scrolled = regionDiff(screenshot, a0End, minis);
      assert.ok(atEnd.compared === 2100 && atEnd.differing === 0, `the bar at the end against A0 06: ${JSON.stringify(atEnd)}`);
      assert.ok(scrolled.compared > 5000 && scrolled.differing === 0, `the scrolled minis against A0 06: ${JSON.stringify(scrolled)}`);
      const frame = resource(source, 10579);
      const selectedChecks = [
        { id: 10579, ...compareIcon(screenshot, selected.cells[9], frame, "tenth-selected-frame", directory) },
      ];
      assertExact(selectedChecks);
      // The bar is a child window over the list (FUN_0060fa80): its up
      // arrow scrolls back a step and leaves the selection alone.
      await click(page, { x: origin[0] + 224, y: origin[1] + 85 });
      const back = defensesWindow(await until(page, "the up arrow scrolls back a step",
        (o, value) => o.defenses_windows.some((w) => w.system_dat_id === value[0] && w.cells[9][1] === value[1]),
        [dat, last[1] + 10]), dat);
      assert.equal(back.selected, 9, JSON.stringify(back));
      return { window: squadrons, stepped: stepped.cells[0], end: end.cells, back: back.cells[9], a0: { top, atEnd, scrolled }, selectedChecks };
    },
  },
  {
    name: "missions-en-route",
    // The second system's agent is in hyperspace to it. FUN_00556430 and
    // FUN_00556390 put it in the target's container at once
    // (mission-lifecycle.md), so its mission lights the icon
    // (FUN_004a1f60) and the window lists it, its mini standing on the
    // en route mark 11501 (FUN_0042c3b0: +0x50 bit 4; the mini keyed over
    // it). The mark is a still bitmap: the manual's "moving starfield"
    // (Figs. 2.35, 3.51) is its streaks, and the window has no timer.
    async run(page, faction, setup, directory, source) {
      const [left, top] = quadrant(setup, "second", "missions");
      const at = { x: Math.round(left) + 24, y: Math.round(top) + 9 };
      await doubleClick(page, at);
      const dat = setup.second_dat_id;
      const opened = await until(page, "the second system's missions icon opens its window",
        (o, value) => o.missions_windows.some((window) => window.system_dat_id === value), dat);
      const window = missionsWindow(opened, dat);
      assert.equal(window.members.length, 1, JSON.stringify(window));
      const [mini, marks] = window.member_art[0];
      // The personnel backdrop 11500 first, then the mark. The backdrop's
      // key pixel shows the window through it, as A0's Defenses page
      // minis do (personnel-states/40, (24, 41, 104) at its bottom left).
      assert.deepEqual(marks, [11500, 11501], JSON.stringify(window));
      assert.ok(mini, JSON.stringify(window));
      const backdrop = resource(source, 11500);
      const mark = resource(source, 11501);
      const face = gokres(mini);
      // The mark and the mini keyed over the backdrop, all at the mini's
      // centred place in the 122 by 43 image box (FUN_0049fef0's integer
      // offsets).
      const composite = new PNG({ width: backdrop.width, height: backdrop.height });
      backdrop.data.copy(composite.data);
      for (const layer of [mark, face]) {
        for (let y = 0; y < Math.min(layer.height, backdrop.height); y += 1) {
          for (let x = 0; x < Math.min(layer.width, backdrop.width); x += 1) {
            const from = (y * layer.width + x) * 4;
            if (!blueKey(layer.data, from)) layer.data.copy(composite.data, (y * backdrop.width + x) * 4, from, from + 4);
          }
        }
      }
      const [cellLeft, cellTop] = window.member_rects[0];
      const place = [cellLeft + Math.trunc((122 - face.width) / 2), cellTop + Math.trunc((43 - face.height) / 2)];
      const first = await stableScreen(page, directory, "en-route");
      const checks = [{ id: 11501, mini, ...compareIcon(first, place, composite, "en-route-mini", directory) }];
      assertExact(checks);
      await page.waitForTimeout(1_500);
      const later = await stableScreen(page, directory, "en-route-later");
      const still = compareIcon(later, place, composite, "en-route-mini-later", directory);
      assert.equal(still.different_pixels, 0, `the mark moved: ${JSON.stringify(still)}`);
      // The whole window against A0 mission-reports/missions-window-enroute-
      // day276 (the Empire's "Mission at Chandrila" at (153, 91)): every
      // pixel but the text (title, "Target:", the target's and the agent's
      // names) and what each game's state draws (A0 lists two missions, the
      // second selected; the target's picture). A0 shows 11165's grey under
      // the title buttons' key pixels (FUN_00602d30 -> FUN_005fd0f0), and
      // under the tabs' key pixels 11165 at the tabs' strip positions,
      // (166, 202, 240) at (105, 142) and (90, 88, 94) at (166, 142): each
      // tab copies its backdrop from the window's surface at its own (x, y)
      // in the strip (FUN_0049f540 -> FUN_00602150, FUN_00602d30). No pixel
      // may differ.
      const missionsA0 = path.join(root, ".artifacts/a0-vm/mission-reports/missions-window-enroute-day276.png");
      let whole = null;
      if (faction.side === 2 && fs.existsSync(missionsA0)) {
        whole = wholeWindow(later, window.origin, missionsA0, [153, 91], [
          [19, 2, 180, 14], [5, 24, 90, 100], [109, 25, 113, 14], [108, 37, 122, 50],
          [109, 91, 113, 16], [107, 180, 117, 14],
        ]);
        assert.ok(whole.compared > 45_000 && whole.differing === 0,
          `the whole Missions window: ${JSON.stringify(whole)}`);
      }
      // A0 mission-reports/missions-window-enroute-day276 (the Empire's
      // "Mission at Chandrila", Jerjerrod in hyperspace): the Empire's
      // traveller is Jerjerrod, and his composed mini is found in A0's
      // team list with every pixel A0's.
      let a0 = null;
      const a0File = path.join(path.dirname(fileURLToPath(import.meta.url)), "../..",
        ".artifacts/a0-vm/mission-reports/missions-window-enroute-day276.png");
      if (faction.name === "empire" && fs.existsSync(a0File)) {
        const shot = PNG.sync.read(fs.readFileSync(a0File));
        let best = null;
        for (let top = 225; top < 275; top += 1) {
          for (let left = 260; left < 340; left += 1) {
            let differ = 0;
            for (let y = 0; y < composite.height && (best === null || differ < best.differ); y += 1) {
              for (let x = 0; x < composite.width; x += 1) {
                const i = ((top + y) * shot.width + left + x) * 4;
                const j = (y * composite.width + x) * 4;
                // The backdrop's key pixel shows the window through it.
                if (blueKey(composite.data, j)) continue;
                if ([0, 1, 2].some((c) => shot.data[i + c] !== composite.data[j + c])) differ += 1;
              }
            }
            if (best === null || differ < best.differ) best = { left, top, differ };
          }
        }
        a0 = best;
        assert.equal(best.differ, 0, `Jerjerrod's en route mini differs from A0: ${JSON.stringify(best)}`);
      }
      return { at, window, checks, still, a0, whole };
    },
  },
  {
    name: "missions",
    // FUN_0045aac0 maps kind 0x40 to type 11 (FUN_0049f130): the primary
    // system's two Diplomacy missions, the player's first and selected
    // (FUN_0049f540, FUN_00609500), its agent listed; another row's click
    // reselects (FUN_004a0c60), the Decoys tab refills (id 0x16), and
    // minimize goes to the rail and back (0x466, FUN_004a21c0).
    async run(page, faction, setup, directory, source) {
      const [left, top] = quadrant(setup, "primary", "missions");
      const at = { x: Math.round(left) + 24, y: Math.round(top) + 9 };
      await doubleClick(page, at);
      const dat = setup.primary_dat_id;
      const opened = await until(page, "the missions icon opens the Missions window",
        (o, value) => o.missions_windows.some((window) => window.system_dat_id === value), dat);
      const window = missionsWindow(opened, dat);
      assert.deepEqual(window.rows, [
        ["Diplomacy", faction.side, DIPLOMACY_MINI[faction.side]],
        ["Diplomacy", faction.other, DIPLOMACY_MINI[faction.other]],
      ], JSON.stringify(window));
      assert.equal(window.side, faction.other);
      assert.deepEqual([window.selected, window.tab, window.tab_side], [0, "agents", faction.side]);
      assert.equal(window.members.length, 1, JSON.stringify(window));
      assert.ok(window.target && window.target !== "Target Unknown", JSON.stringify(window));
      const [rowLeft, rowTop] = window.row_rects[0];
      const origin = [rowLeft - 5, rowTop - 24];
      const background = resource(source, 11165);
      let screenshot = await stableScreen(page, directory, "missions");
      const other = DIPLOMACY_MINI[faction.other];
      const opening = [
        // Below the two rows the list shows the background, as does the
        // foot right of the list.
        { id: 11165, ...compareRegion(screenshot, origin, background, [5, 124, 94, 175], "missions-list", directory) },
        { id: 11165, ...compareRegion(screenshot, origin, background, [100, 292, 135, 12], "missions-foot", directory) },
        { id: other, ...compareRowMini(screenshot, window.row_rects[1], gokres(other), "missions-second-mini", directory) },
        { id: MISSION_FRAME[faction.side], ...compareIcon(screenshot, window.row_rects[0],
          resource(source, MISSION_FRAME[faction.side]), "missions-selected-frame", directory) },
        ...compareMissionTabs(screenshot, source, window, directory, "missions"),
      ];
      assertExact(opening);

      const second = window.row_rects[1];
      await click(page, { x: second[0] + 40, y: second[1] + 20 });
      const reselected = missionsWindow(await until(page, "a click selects the other mission",
        (o, value) => o.missions_windows.some((w) => w.system_dat_id === value && w.selected === 1), dat), dat);
      assert.deepEqual([reselected.tab, reselected.tab_side], ["agents", faction.other]);
      assert.equal(reselected.members.length, 1, JSON.stringify(reselected));
      screenshot = await stableScreen(page, directory, "reselected");
      const own = DIPLOMACY_MINI[faction.side];
      const reselectedChecks = [
        { id: own, ...compareRowMini(screenshot, reselected.row_rects[0], gokres(own), "reselected-first-mini", directory) },
        { id: MISSION_FRAME[faction.side], ...compareIcon(screenshot, reselected.row_rects[1],
          resource(source, MISSION_FRAME[faction.side]), "reselected-frame", directory) },
        ...compareMissionTabs(screenshot, source, reselected, directory, "reselected"),
      ];
      assertExact(reselectedChecks);

      const decoysTab = reselected.tabs.find(([name]) => name === "decoys")[1];
      await click(page, { x: decoysTab[0] + 30, y: decoysTab[1] + 8 });
      const decoys = missionsWindow(await until(page, "the Decoys tab refills the member list",
        (o, value) => o.missions_windows.some((w) => w.system_dat_id === value && w.tab === "decoys"), dat), dat);
      assert.deepEqual(decoys.members, []);
      screenshot = await stableScreen(page, directory, "decoys");
      const decoyChecks = compareMissionTabs(screenshot, source, decoys, directory, "decoys");
      assertExact(decoyChecks);
      // FUN_0049f540: Agents and Decoys have their tips.
      for (const [name, rect] of decoys.tabs) {
        assert.ok(await tipShows(page, { x: Math.round(rect[0] + 30), y: Math.round(rect[1] + 8) }),
          `the ${name} tab shows no tip`);
      }

      await click(page, { x: origin[0] + 210, y: origin[1] + 9 });
      const railed = await until(page, "minimize sends the window to the rail",
        (o, value) => !o.missions_windows.some((w) => w.system_dat_id === value)
          && o.rail.some(([system, kind]) => system === value && kind === "missions"), dat);
      const slot = railed.rail.find(([system, kind]) => system === dat && kind === "missions")[2];
      screenshot = await stableScreen(page, directory, "railed");
      const icon = MISSIONS_RAIL[faction.other];
      const railChecks = [{ id: icon, ...compareIcon(screenshot, slot, resource(source, icon), "rail-icon", directory) }];
      assertExact(railChecks);

      // The rail slot restores on a double click (FUN_00422ce0 0x203).
      await doubleClick(page, { x: slot[0] + slot[2] / 2, y: slot[1] + slot[3] / 2 });
      const restored = missionsWindow(await until(page, "the rail slot restores the window",
        (o, value) => o.missions_windows.some((w) => w.system_dat_id === value)
          && !o.rail.some(([system, kind]) => system === value && kind === "missions"), dat), dat);
      // FUN_00428b40 keeps the window's object in its rail slot, for every
      // window kind: it comes back as it was minimized, with the second
      // mission and the Decoys tab (A0 personnel-states/39, the Fleet
      // window back with its selection).
      assert.deepEqual(restored.origin, window.origin);
      assert.deepEqual([restored.selected, restored.tab], [1, "decoys"]);
      // The icon's double click takes a railed window back too, as the
      // Defenses icon's does (A0 personnel-states/38).
      await page.mouse.move(2, 2);
      await page.waitForTimeout(500);
      await click(page, { x: origin[0] + 210, y: origin[1] + 9 });
      await until(page, "minimize sends the window to the rail again",
        (o, value) => !o.missions_windows.some((w) => w.system_dat_id === value)
          && o.rail.some(([system, kind]) => system === value && kind === "missions"), dat);
      await page.mouse.move(2, 2);
      await frames(page, 4);
      await page.waitForTimeout(1_000);
      await doubleClick(page, at);
      const fromIcon = missionsWindow(await until(page, "the icon takes the window off the rail",
        (o, value) => o.missions_windows.some((w) => w.system_dat_id === value)
          && !o.rail.some(([system, kind]) => system === value && kind === "missions"), dat), dat);
      assert.deepEqual([fromIcon.selected, fromIcon.tab, fromIcon.origin], [1, "decoys", window.origin],
        JSON.stringify(fromIcon));

      // The first mission's menu (Encyclopedia, Status, Abort) and its
      // Status (FUN_00445c60): A0 missionstatus/02, 03 (the Alliance's
      // Espionage at Selonia, Status at (108, 79)): "Mission Status" over
      // Target, ETA Destination (en route only), Team Size and Decoys. The
      // fixture's Diplomacy is not en route, so its rows are Target, Team
      // Size and Decoys; the labels' left edge and the title's box are
      // compared with A0's.
      const first = restored.row_rects[0];
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
      await rightClick(page, { x: first[0] + 40, y: first[1] + 20 });
      await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
        null, { timeout: 10_000 });
      const missionMenu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
      assert.deepEqual(missionMenu.row_states.map(([label]) => label), ["Encyclopedia", "Status", "Abort"],
        JSON.stringify(missionMenu));
      const rowHeight = (missionMenu.height - 2) / missionMenu.rows;
      await click(page, { x: missionMenu.left + missionMenu.width / 2,
        y: missionMenu.top + 2 + rowHeight * (missionMenu.status_window_row + 0.5) });
      await page.mouse.move(2, 2);
      await page.waitForTimeout(500);
      screenshot = await stableScreen(page, directory, "mission-status");
      let missionStatus = null;
      const a0File = path.join(root, ".artifacts/a0-vm/missionstatus/03-mission-status.png");
      if (faction.side === 1 && fs.existsSync(a0File)) {
        const a0 = PNG.sync.read(fs.readFileSync(a0File));
        const port = screenshot;
        const ink = (png, [x0, y0, x1, y1]) => {
          let box = null;
          for (let y = y0; y < y1; y += 1) {
            for (let x = x0; x < x1; x += 1) {
              const i = (y * png.width + x) * 4;
              if (Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) <= 200) continue;
              box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
            }
          }
          return box;
        };
        const title = [108 + 15, 79 + 18, 108 + 226, 79 + 36];
        const target = [108 + 16, 79 + 47, 108 + 118, 79 + 61];
        missionStatus = { title: { a0: ink(a0, title), port: ink(port, title) },
          target: { a0: ink(a0, target), port: ink(port, target) } };
        for (const [name, { a0: a, port: p }] of Object.entries(missionStatus)) {
          // A leading "T" stands up to 3 pixels right in Liberation Sans
          // (the accepted text deviation, as the ship's Trooper row).
          assert.ok(a && p && Math.abs(a[0] - p[0]) <= 3 && [1, 3].every((k) => Math.abs(a[k] - p[k]) <= 2)
            && Math.abs(a[2] - p[2]) <= 4,
            `mission status ${name}: ${JSON.stringify(missionStatus)}`);
        }
      }
      return { at, window, opening, reselected, reselectedChecks, decoyChecks, railChecks, restored, fromIcon, missionMenu, missionStatus };
    },
  },
  {
    name: "headquarters-move-a0",
    // OBJ-13-C005/C006 against A0 2026-11-13-hq-move (the original moving
    // the Alliance HQ from Jerijador to Yavin): see headquartersMoveA0.
    factions: ["alliance"],
    run: (page, faction, setup, directory, source) => headquartersMoveA0(page, setup, directory, source),
  },
];

// A0 2026-11-13-hq-move (REBEXE in the offline VM, an Alliance game, days
// 346-347, README there): 00 the HQ overlay's menu at Jerijador, opened at
// (248, 230); 02 just after Move's target click on Yavin, with no
// confirmation; 03 the HQ's menu at Yavin en route, opened at (404, 232);
// 04 its Defense Facility Status, the window at (108, 79).
const HQ_MOVE_A0 = path.join(root, ".artifacts/a0-vm/2026-11-13-hq-move");
const HQ_MOVE_A0_SHA256 = {
  "00-hq-overlay-menu-jerijador.png": "9565c0581140c2deed8f5cc4554871771ef221712970c2bb8e2be0a098ce2e89",
  "02-after-target-click-on-yavin.png": "5d3aa4a67f2247638b31474b1ea7e8be0ab3681ace06745f0133a0fe8c60d3b0",
  "03-hq-menu-at-yavin-en-route.png": "5a9410e9b5edd1db322b6532dbc7bf4769d91a78912ecda92d7368ee89bf2135",
  "04-hq-status-en-route.png": "b855840545c5ff267bf2f4a84768c46448c65b060c20e66a8c23d371a9f5227d",
  "05-final-state-paused.png": "97510ad926deb71556d6337cd943433cd20115c5c30120cc476b2ba903b89540",
};

function readA0(name) {
  const bytes = fs.readFileSync(path.join(HQ_MOVE_A0, name));
  assert.equal(sha256(bytes), HQ_MOVE_A0_SHA256[name], `A0 ${name} changed`);
  return PNG.sync.read(bytes);
}

function rgb(png, x, y) {
  const i = (y * png.width + x) * 4;
  return [png.data[i], png.data[i + 1], png.data[i + 2]];
}

// A pop-up menu of `rows` 20-pixel rows (FUN_004abbf0), 104 by 82 from
// its opening point, on `port` at `portOrigin` against A0's at
// `a0Origin`. The frame (FUN_00607740 from STRATEGY 10100..10107, passed
// by FUN_00442860: a (192,192,192) outer line, a white inner one, the
// sides' tiles on every second row over the window's fill; cockpit.rs
// paint_gid_frame_border) must class every pixel as A0 does: the outer
// grey, the inner white or neither. Each row's text ink
// (white enabled, (128,128,128) disabled: FUN_004aba60 +0x30 / +0x34)
// must lie within a pixel of A0's. A0's pointer covers the frame's top
// left and the first row's first two letters, so those stay out.
function compareMenuWithA0(port, portOrigin, a0, a0Origin, enabled) {
  const [width, height] = [104, 82];
  const pointer = (x, y) => x < 22 && y < 24;
  const kind = ([r, g, b]) => (r === 192 && g === 192 && b === 192 ? "outer"
    : r === 255 && g === 255 && b === 255 ? "inner" : "behind");
  const frame = { compared: 0, differing: [] };
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      if (!(x < 2 || y < 2 || x >= width - 2 || y >= height - 2) || pointer(x, y)) continue;
      frame.compared += 1;
      const [a, p] = [kind(rgb(a0, a0Origin[0] + x, a0Origin[1] + y)), kind(rgb(port, portOrigin[0] + x, portOrigin[1] + y))];
      if (a !== p) frame.differing.push([x, y, a, p]);
    }
  }
  const ink = (png, origin, [x0, y0, x1, y1], test) => {
    let box = null;
    for (let y = y0; y < y1; y += 1) for (let x = x0; x < x1; x += 1) {
      if (!test(rgb(png, origin[0] + x, origin[1] + y))) continue;
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
    }
    return box;
  };
  const white = (c) => Math.min(...c) > 200;
  // A0's disabled text is (128,128,128) exactly; the port's glyphs are
  // smoothed over the fill (the accepted text deviation), so their core
  // reads 100..128 neutral grey.
  const grey = ([r, g, b]) => r === g && g === b && r >= 100 && r <= 128;
  const rows = enabled.map((on, row) => {
    // Inside the frame: the last row's box stops above the bottom line.
    const region = [row === 0 ? 22 : 2, 2 + 20 * row, width - 2, Math.min(22 + 20 * row, height - 2)];
    const [shown, other] = on ? [white, grey] : [grey, white];
    return {
      enabled: on,
      a0: ink(a0, a0Origin, region, shown),
      port: ink(port, portOrigin, region, shown),
      a0_other: ink(a0, a0Origin, region, other),
      port_other: ink(port, portOrigin, region, other),
    };
  });
  return { frame: { compared: frame.compared, differing: frame.differing.length, first: frame.differing.slice(0, 8) }, rows };
}

function assertMenuMatchesA0(check, label) {
  assert.ok(check.frame.compared > 300 && check.frame.differing === 0, `${label} frame: ${JSON.stringify(check.frame)}`);
  check.rows.forEach((row, index) => {
    assert.ok(row.a0 && row.port && row.a0.every((value, k) => Math.abs(value - row.port[k]) <= 1),
      `${label} row ${index} ink: ${JSON.stringify(row)}`);
    // A0 draws the row in one colour; the menu's fill shows what lies
    // behind it, so the port's other-colour pixels (the scene) are only
    // reported.
    assert.equal(row.a0_other, null, `${label} row ${index} colour: ${JSON.stringify(row)}`);
  });
}

// Where `image`'s opaque pixels (palette-blue key) best match `png` within
// `radius` of `around`, skipping `skip(x, y)` image pixels: the place and
// the differing pixels there.
function bestMatch(png, image, around, radius, skip = () => false) {
  let best = null;
  for (let dy = -radius; dy <= radius; dy += 1) {
    for (let dx = -radius; dx <= radius; dx += 1) {
      const [left, top] = [around[0] + dx, around[1] + dy];
      const differing = [];
      for (let y = 0; y < image.height; y += 1) for (let x = 0; x < image.width; x += 1) {
        const from = (y * image.width + x) * 4;
        if (blueKey(image.data, from) || skip(x, y)) continue;
        const c = rgb(png, left + x, top + y);
        if (c.some((value, k) => value !== image.data[from + k])) differing.push([x, y, ...c]);
      }
      if (!best || differing.length < best.differing.length) best = { at: [left, top], differing };
    }
  }
  return { at: best.at, differing: best.differing.length, first: best.differing.slice(0, 24), all: best.differing };
}

async function openHeadquartersMenu(page, at) {
  await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
  await rightClick(page, at);
  await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
    null, { timeout: 10_000 });
  return page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
}

async function headquartersMoveA0(page, setup, directory, source) {
  const result = {};
  const icon = resource(source, 904);
  // 00: Move, Confirmed Move, Encyclopedia and Status, all white
  // (FUN_00526c00, FUN_0051d990; FUN_00578c00 enables both moves at rest).
  const at = { x: Math.round(setup.headquarters[0]) + 20, y: Math.round(setup.headquarters[1]) + 12 };
  const menu = await openHeadquartersMenu(page, at);
  assert.deepEqual(menu.row_states,
    [["Move", true], ["Confirmed Move", true], ["Encyclopedia", true], ["Status", true]], JSON.stringify(menu));
  assert.deepEqual([menu.left, menu.top, menu.width, menu.height], [at.x, at.y, 104, 82], JSON.stringify(menu));
  const menuScreen = await stableScreen(page, directory, "a0-headquarters-menu");
  result.menu = compareMenuWithA0(menuScreen, [menu.left, menu.top], readA0("00-hq-overlay-menu-jerijador.png"),
    [248, 230], [true, true, true, true]);
  assertMenuMatchesA0(result.menu, "the HQ menu (A0 00)");

  // 02: Move (0x201), then the target: no confirmation (FUN_00487cc0 asks
  // only for Confirmed Move or a blockade), and 904 stands at once over the
  // destination's planet, its top left on the planet's (FUN_0045bbb0), and
  // no longer over the origin (FUN_004fbc20's notice 0x300, FUN_00486d80).
  const height = (menu.height - 2) / menu.rows;
  await click(page, { x: menu.left + menu.width / 2, y: menu.top + 2 + height * (menu.move_row + 0.5) });
  await frames(page, 2);
  await click(page, { x: Math.round(setup.uprising[0]) + 18, y: Math.round(setup.uprising[1]) + 18 });
  await page.waitForTimeout(600);
  const moved = await stableScreen(page, directory, "a0-headquarters-moved");
  const port = {
    destination: compareIcon(moved, setup.uprising, icon, "a0-headquarters-destination", directory),
    origin: compareIcon(moved, setup.headquarters, icon, "a0-headquarters-origin", directory),
  };
  assert.equal(port.destination.different_pixels, 0, JSON.stringify(port));
  assert.ok(port.origin.different_pixels > port.origin.pixels_checked / 2, JSON.stringify(port));
  const a0Moved = readA0("02-after-target-click-on-yavin.png");
  // A0's 904 by Yavin; the pointer (white and black) covers a few pixels.
  const mark = bestMatch(a0Moved, icon, [389, 217], 6);
  const pointerOnly = mark.all.every(([, , r, g, b]) => (r === 255 && g === 255 && b === 255) || (r + g + b === 0));
  // Yavin's planet, STRATEGY 10231, under 904's opaque pixels, in A0 05
  // (day 348, the same view with the pointer away; 904 there exactly).
  const a0Later = readA0("05-final-state-paused.png");
  const later = bestMatch(a0Later, icon, mark.at, 0);
  const planet = bestMatch(a0Later, resource(source, 10231), mark.at, 6,
    (x, y) => x < icon.width && y < icon.height && !blueKey(icon.data, (y * icon.width + x) * 4));
  const left = bestMatch(a0Moved, icon, [230, 214], 0);
  result.moved = { port, a0: { mark: { at: mark.at, differing: mark.differing, pointer_only: pointerOnly },
    later: { differing: later.differing }, planet: { at: planet.at, differing: planet.differing },
    origin: { differing: left.differing } } };
  assert.ok(mark.differing <= 24 && pointerOnly, `A0 904 by Yavin: ${JSON.stringify(result.moved.a0)}`);
  assert.equal(later.differing, 0, `A0 05 904 by Yavin: ${JSON.stringify(result.moved.a0)}`);
  assert.equal(planet.differing, 0, `A0 Yavin's planet: ${JSON.stringify(result.moved.a0)}`);
  // 904's top left on its planet's in A0 as in the port (setup.uprising
  // is the destination's planet rect).
  assert.deepEqual(mark.at, planet.at, JSON.stringify(result.moved.a0));
  assert.ok(left.differing > icon.width * icon.height / 4, `A0 904 left Jerijador: ${JSON.stringify(result.moved.a0)}`);

  // 03: en route, Move and Confirmed Move grey (FUN_004f9860 4).
  const there = { x: Math.round(setup.uprising[0]) + 20, y: Math.round(setup.uprising[1]) + 12 };
  const enRoute = await openHeadquartersMenu(page, there);
  assert.deepEqual(enRoute.row_states,
    [["Move", false], ["Confirmed Move", false], ["Encyclopedia", true], ["Status", true]], JSON.stringify(enRoute));
  const enRouteScreen = await stableScreen(page, directory, "a0-headquarters-en-route-menu");
  result.enRoute = compareMenuWithA0(enRouteScreen, [enRoute.left, enRoute.top], readA0("03-hq-menu-at-yavin-en-route.png"),
    [404, 232], [false, false, true, true]);
  assertMenuMatchesA0(result.enRoute, "the en route HQ menu (A0 03)");

  // 04: Defense Facility Status (FUN_00444e20) leads with "ETA
  // Destination: Day N" (FUN_004fd2b0: the move's day plus the trip at
  // the GNPRTB 1 speed, A0 346 + 87 = 433), then Location, "Enroute"
  // (FUN_0044a470), Maintenance Cost 0 and Bombardment Defense Strength
  // 9; the HQ's picture over the en route streaks 11506 (FUN_0042c3b0).
  const rowHeight = (enRoute.height - 2) / enRoute.rows;
  await page.evaluate(() => { window.__openRebellionInterfaceStatusRows = undefined; });
  await click(page, { x: enRoute.left + enRoute.width / 2, y: enRoute.top + 2 + rowHeight * (enRoute.status_window_row + 0.5) });
  await page.waitForFunction(() => window.__openRebellionInterfaceStatusRows?.rows?.length, null, { timeout: 10_000 });
  const status = await page.evaluate(() => window.__openRebellionInterfaceStatusRows);
  const statusScreen = await stableScreen(page, directory, "a0-headquarters-en-route-status");
  assert.equal(status.title, "Defense Facility Status", JSON.stringify(status));
  assert.deepEqual(status.rows.map(([label]) => label),
    ["ETA Destination:", "Location:", "Status:", "Maintenance Cost:", "Bombardment Defense Strength:"], JSON.stringify(status));
  assert.ok(Number.isInteger(setup.headquarters_trip_days) && setup.headquarters_trip_days > 0, JSON.stringify(setup));
  assert.equal(status.rows[0][1], `Day ${status.today + setup.headquarters_trip_days}`, JSON.stringify(status));
  assert.deepEqual([status.rows[2][1], status.rows[3][1], status.rows[4][1]], ["Enroute", "0", "9"], JSON.stringify(status));
  assert.deepEqual(status.marks, [["Strategy", 11506]], JSON.stringify(status));
  const a0Status = readA0("04-hq-status-en-route.png");
  const [x0, y0] = [108, 79];
  // Differing pixels, and those of them A0's pointer (pure white or
  // black) does not explain: A0 04's pointer rests on the right button.
  const differing = ([xa, ya, xb, yb]) => {
    let count = 0;
    let other = 0;
    for (let y = ya; y < yb; y += 1) for (let x = xa; x < xb; x += 1) {
      const a = rgb(a0Status, x0 + x, y0 + y);
      if (!a.some((value, k) => value !== rgb(statusScreen, x0 + x, y0 + y)[k])) continue;
      count += 1;
      if (!a.every((value) => value === 255) && !a.every((value) => value === 0)) other += 1;
    }
    return { count, other };
  };
  const ink = (png, [xa, ya, xb, yb]) => {
    let box = null;
    for (let y = ya; y < yb; y += 1) for (let x = xa; x < xb; x += 1) {
      if (Math.min(...rgb(png, x0 + x, y0 + y)) <= 200) continue;
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
    }
    return box;
  };
  result.status = { rows: status.rows, today: status.today, trip_days: setup.headquarters_trip_days,
    picture: differing([242, 15, 372, 113]), buttons: differing([242, 200, 372, 260]), ink: {} };
  assert.equal(result.status.picture.count, 0, JSON.stringify(result.status));
  assert.equal(result.status.buttons.other, 0, JSON.stringify(result.status));
  // Six label lines 14 apart ("Bombardment" / "Defense Strength:" wrap),
  // the title and the name; the values Enroute, 0 and 9; of "Day N" its
  // left, top and bottom ("Day " leads). Location names another system.
  const regions = { title: [15, 18, 226, 36], name: [242, 137, 372, 181] };
  for (let k = 0; k < 6; k += 1) regions[`label-${k}`] = [16, 47 + 14 * k, 118, 61 + 14 * k];
  for (const k of [2, 3, 5]) regions[`value-${k}`] = [119, 47 + 14 * k, 210, 61 + 14 * k];
  regions["value-0"] = [119, 47, 210, 61];
  for (const [name, region] of Object.entries(regions)) {
    const [a, b] = [ink(a0Status, region), ink(statusScreen, region)];
    result.status.ink[name] = { a0: a, port: b };
    const compared = name === "value-0" ? [0, 1, 3] : [0, 1, 2, 3];
    assert.ok(a && b && compared.every((k) => Math.abs(a[k] - b[k]) <= 1), `${name}: ${JSON.stringify(result.status.ink[name])}`);
  }
  return result;
}

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

    const fixtureCode = (testCase.scenario ?? QUADRANTS) | (faction.byte << 8);
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
    assert.deepEqual(
      [start.system_windows, start.defenses_windows, start.missions_windows, start.rail],
      [[], [], [], []],
      JSON.stringify(start));
    await page.evaluate(() => document.fonts.ready);
    const { bytes: _ready, ...before } = await shot(page, directory, "ready");

    const checks = await testCase.run(page, faction, setup, directory, source, consoleLines);
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
  const gokresIds = [...usedGokres].sort((left, right) => left - right);
  const gokresAggregate = createHash("sha256");
  for (const id of gokresIds) {
    gokresAggregate.update(`${id}\0`);
    gokresAggregate.update(fs.readFileSync(path.join(gokresDirectory(), `${id}.bmp`)));
  }
  return [
    { dll: "STRATEGY.DLL", resource_ids: ids, aggregate_sha256: aggregate.digest("hex") },
    { dll: "GOKRES.DLL", resource_ids: gokresIds, aggregate_sha256: gokresAggregate.digest("hex") },
  ];
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
        if (testCase.factions && !testCase.factions.includes(faction.name)) continue;
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
    scope: "test-only sector window quadrant icons (FUN_00459e30): each shown icon's art against STRATEGY.DLL, a pressed icon's selected art, hidden icons absent, the system icon opening the System window, and the defenses icon opening the System Defenses window (tabs, rows, selection frame, rail), and the missions icon opening the Missions window (rows, selection frame, tabs, rail) against STRATEGY.DLL and GOKRES.DLL, on both sides",
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
