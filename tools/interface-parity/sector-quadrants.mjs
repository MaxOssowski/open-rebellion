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
      if (keyed && blueKey(icon.data, from)) {
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
async function systemWindowText(page, origin) {
  await page.mouse.move(2, 470);
  await page.waitForTimeout(800);
  const port = PNG.sync.read(await page.screenshot());
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
  return checks;
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
    checks.push({ id, ...compareIcon(screenshot, rect, resource(source, id), `${prefix}-tab-${name}`, directory) });
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
      return { compares, absent, headquarters, uprising };
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
      const textChecks = faction.side === 1 && a0Dir ? await systemWindowText(page, origin) : null;
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
      // window at (305, 86)), its first line only: the port draws "Garrison
      // Requirement" under it (FUN_004a90d0 for the holder's player), which
      // A0's Alliance windows (Mon Calamari, Pzob) do not show.
      await page.mouse.move(2, 2);
      await page.waitForTimeout(300);
      checkLabel("regiments", await stableScreen(page, directory, "regiments-label"),
        path.join(root, ".artifacts/a0-vm/personnel-states/15-mon-calamari-regiments.png"), [305, 86], 66);
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
    // window's selection is kept (sector-icon-menus.md): the right release
    // lists that selection's orders (FUN_004ac5c0, FUN_0051d990). With
    // nothing selected, Encyclopedia (one object) and Status are both
    // disabled. After a press selects the primary's mission icon, a right
    // click on another planet shows the mission icon's menu, as A0
    // menus/10-planet-right.png shows the kept fleet icon's menu.
    async run(page, faction, setup, directory) {
      const [left, top, width, height] = setup.headquarters;
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
      return { at, window, checks };
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
      return { at, window, checks, still };
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
    assert.deepEqual(
      [start.system_windows, start.defenses_windows, start.missions_windows, start.rail],
      [[], [], [], []],
      JSON.stringify(start));
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
