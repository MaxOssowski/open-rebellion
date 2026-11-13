#!/usr/bin/env node

// The Status window (window type 0x1a, FUN_00442d70; ghidra/notes/status-window.md)
// opened from each object family's pop-up menu (Status, 0x103): a character
// (manual p. 101: the System Defenses window's personnel page), a regiment
// (its regiment page), a fleet and a capital ship (the Fleet window) and a
// production manager (a Manufacturing window band). Its STRATEGY background
// and buttons are compared exactly; the title, list, name and picture are
// masked and checked for content. Close closes each; for the character,
// Escape and the Encyclopedia button close it too. Both sides.

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
// `--a0=DIR` (.artifacts/a0-vm/status) compares the Alliance's Fleet Status
// text with A0 06-fleet, whose window stands at (170, 81) (its Close button
// matches the port's there).
const a0Dir = process.argv.find((arg) => arg.startsWith("--a0="))?.slice(5);
const dimensions = { width: 379, height: 272 };
// Fixture 45 (Scenario::MissionTargeting + 1) opens a Defenses window with the
// player's agent first on its personnel page; `galaxy` is the galaxy view's top-left and width at 640 by
// 480. FUN_00443130: an own-side character's background is 0x2d22 + its side,
// plus 3 for a player not of side 1.
const factions = [
  { name: "alliance", byte: 1, galaxy: { x: 55, y: 40, width: 485 }, background: 11554 },
  { name: "empire", byte: 2, galaxy: { x: 120, y: 40, width: 480 }, background: 11558 },
];
// Fixture codes are the Scenario index plus one (interface_test_fixture.rs):
// 45 MissionTargeting, 48 FleetLoad, 55 BuildSelection.
const MISSION_TARGETING = 45;
// 93 ForceStatus: MissionTargeting with a Force user for the agent.
const FORCE_STATUS = 94;
const FLEET_LOAD = 48;
const BUILD_SELECTION = 55;
const SQUADRON_LOAD = 65;
const BATTERY_STATUS = 92;
const RECON_TARGETING = 63;
// 104 SpecialForceEnroute: ReconTargeting with an Infiltrators team (the
// Empire's Imperial Commandos) travelling to the primary system.
const SPECIAL_FORCE_ENROUTE = 104;
// 110 SquadronEnroute: the primary system's Fighter Squadrons page with an
// A-wing (TIE Fighter) squadron travelling there by Move.
const SQUADRON_ENROUTE = 110;
// 111 FleetArrivals: SquadronLoad's fleet with a regiment, a squadron and a
// character on their way into it by Move, its hold empty.
const FLEET_ARRIVALS = 111;
// 105 RegimentEnroute: the primary system's Trooper Regiments page with an
// Alliance (Imperial) Army Regiment travelling there, then one in training.
const REGIMENT_ENROUTE = 105;
const regimentA0 = "../2026-11-11-regiment-en-route";
// FUN_00443020: 0x66 Encyclopedia and 0x65 Close, 32 by 31.
const buttons = {
  encyclopedia: { x: 258, y: 218, id: 11552 },
  close: { x: 324, y: 218, id: 10370 },
};
// Dynamic content: the title, the list, the name and the picture frame's
// interior (x 242..371, y 15..112 of the owned backgrounds), which a fleet's
// or a production manager's picture fills.
const masks = {
  title: { x: 15, y: 18, width: 211, height: 18 },
  list: { x: 18, y: 47, width: 208, height: 204 },
  name: { x: 242, y: 137, width: 130, height: 44 },
  picture: { x: 242, y: 15, width: 130, height: 98 },
};
const runId = `${new Date().toISOString().replace(/[:.]/g, "-")}-${process.pid}`;
const runDir = path.join(root, ".artifacts/interface-parity", `status-window-${runId}`);

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

function sourceDirectory() {
  const candidates = [
    process.env.REBELLION_STRATEGY_BMP_DIR,
    path.join(root, "data/base/ui/strategy-dll/BMP"),
  ].filter(Boolean);
  const directory = candidates.find((candidate) => fs.existsSync(path.join(candidate, "11554.bmp")));
  if (!directory) throw new Error("owned STRATEGY.DLL BMP extraction is unavailable; set REBELLION_STRATEGY_BMP_DIR");
  return directory;
}

function gokresDirectory() {
  const candidates = [
    process.env.REBELLION_GOKRES_BMP_DIR,
    path.join(root, "data/base/ui/gokres-dll/BMP"),
  ].filter(Boolean);
  const directory = candidates.find((candidate) => fs.existsSync(path.join(candidate, "17472.bmp")));
  if (!directory) throw new Error("owned GOKRES.DLL BMP extraction is unavailable; set REBELLION_GOKRES_BMP_DIR");
  return directory;
}

function browserExecutable() {
  const candidates = [
    ...(process.env.OPEN_REBELLION_CHROME_FOR_TESTING ? [process.env.OPEN_REBELLION_CHROME_FOR_TESTING] : []),
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
    const relative = path.posix.normalize(decodeURIComponent(pathname)).replace(/^\/+/, "") || "index.html";
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
      response.writeHead(200, { "content-type": mimeType(candidate), "content-length": bytes.length, "cache-control": "no-store" }).end(bytes);
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

const resource = (source, id, width, height) => decodeIndexedBmp(fs.readFileSync(path.join(source, `${id}.bmp`)), width, height);

// Buttons blit keyed (FUN_00602d30): their pure-blue mat shows the
// background, as the A0 Character Status shows around Close (2026-10-29,
// .artifacts/a0-vm/status/15-char-veers.png).
function blitKeyed(destination, image, x, y) {
  for (let row = 0; row < image.height; row += 1) {
    for (let column = 0; column < image.width; column += 1) {
      const from = (row * image.width + column) * 4;
      if (image.data[from] === 0 && image.data[from + 1] === 0 && image.data[from + 2] === 255) continue;
      const to = ((y + row) * destination.width + x + column) * 4;
      destination.data.set(image.data.subarray(from, from + 4), to);
    }
  }
}

function composeExpected(source, faction) {
  const expected = resource(source, faction.background, dimensions.width, dimensions.height);
  for (const button of Object.values(buttons)) blitKeyed(expected, resource(source, button.id, 32, 31), button.x, button.y);
  return expected;
}

const inside = (box, x, y) => x >= box.x && x < box.x + box.width && y >= box.y && y < box.y + box.height;

function crop(screenshotBytes, rect) {
  const screenshot = PNG.sync.read(screenshotBytes);
  assert.deepEqual([screenshot.width, screenshot.height], [640, 480]);
  const window = new PNG({ width: rect.width, height: rect.height });
  for (let y = 0; y < rect.height; y += 1) {
    const start = ((rect.y + y) * screenshot.width + rect.x) * 4;
    window.data.set(screenshot.data.subarray(start, start + rect.width * 4), y * rect.width * 4);
  }
  return window;
}

function compare(actual, expected, directory, label) {
  let different = 0;
  let checked = 0;
  const content = Object.fromEntries(Object.keys(masks).map((name) => [name, 0]));
  let blue = 0;
  for (let y = 0; y < expected.height; y += 1) {
    for (let x = 0; x < expected.width; x += 1) {
      const offset = (y * expected.width + x) * 4;
      const same = [0, 1, 2].every((index) => actual.data[offset + index] === expected.data[offset + index]);
      const mask = Object.entries(masks).find(([, box]) => inside(box, x, y));
      if (mask) {
        if (!same) content[mask[0]] += 1;
        if (mask[0] === "picture" && actual.data[offset] === 0 && actual.data[offset + 1] === 0 && actual.data[offset + 2] === 255) blue += 1;
        continue;
      }
      checked += 1;
      if (!same) different += 1;
    }
  }
  fs.writeFileSync(path.join(directory, `${label}-actual.png`), PNG.sync.write(actual));
  fs.writeFileSync(path.join(directory, `${label}-expected.png`), PNG.sync.write(expected));
  return { pixels_checked: checked, different_pixels: different, changed_in_masks: content, picture_blue_pixels: blue };
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

// Right-click `at`, wait for the pop-up menu and choose its Status row.
// Rows below a 2-pixel border share one height (object_menu.rs).
async function chooseStatus(page, at) {
  await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
  await click(page, at, "right");
  await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu", null, { timeout: 10_000 });
  const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
  assert.notEqual(menu.status_window_row, null, "the menu lists Status");
  const row = (menu.height - 2) / menu.rows;
  await click(page, { x: menu.left + menu.width / 2, y: menu.top + 2 + row * (menu.status_window_row + 0.5) });
  return menu;
}

const point = ([x, y]) => ({ x: Math.round(x), y: Math.round(y) });

// What fleet-ship-en-route's and fleet-status-en-route's targets read
// from the Fleet window: the Fighters and Troops counters and the
// Personnel list's characters, or all its members for the fleet.
let fleetShipEmbarked = null;

// The fixture's special force on its way in (SPECFCSD class 2 or 7).
const isSpecialForce = (item) => /Infiltrators|Commandos/.test(item);

// The Fleet window's Fighters and Troops counters and Personnel list,
// with the fleet selected, back on its Capital Ships tab.
async function fleetEmbarked(page) {
  const opened = await openFleetWindow(page);
  await click(page, point(opened.fleet_entry));
  const ships = await fleetLoad(page, (o) => o.selected === "fleet" && o.tab === "CapitalShips"
    && o.first_item && o.troops_tab && o.fighters_tab);
  assert.equal(ships.items.length, 1, JSON.stringify(ships));
  await click(page, point(ships.fighters_tab));
  const fighters = await fleetLoad(page, (o) => o.tab === "Fighters" && o.counts);
  await click(page, point(ships.troops_tab));
  const troops = await fleetLoad(page, (o) => o.tab === "Troops" && o.counts);
  await click(page, { x: Math.round(ships.troops_tab[0] + 32), y: Math.round(ships.troops_tab[1]) });
  const personnel = await fleetLoad(page, (o) => o.tab === "Personnel" && o.items.length > 0);
  // Each traveller is one of these (fleet-squadron-en-route).
  assert.ok(fighters.counts[0] > 0 && troops.counts[0] > 0, JSON.stringify([fighters, troops]));
  assert.equal(personnel.items.filter(isSpecialForce).length, 1, JSON.stringify(personnel));
  const step = ships.troops_tab[0] - ships.fighters_tab[0];
  await click(page, { x: Math.round(ships.fighters_tab[0] - step), y: Math.round(ships.fighters_tab[1]) });
  const back = await fleetLoad(page, (o) => o.tab === "CapitalShips" && o.first_item && o.fleet_entry);
  return { counts: [String(fighters.counts[0]), String(troops.counts[0])], personnel: personnel.items, back };
}

async function doubleClick(page, at) {
  await page.mouse.move(at.x, at.y);
  await frames(page);
  await page.mouse.dblclick(at.x, at.y);
  await frames(page);
}

async function fleetLoadSetup(page) {
  await page.waitForFunction(() => window.__openRebellionInterfaceFleetLoadSetup
    && (window.__openRebellionInterfaceFleetLoads || []).length > 0, null, { timeout: 10_000 });
  return page.evaluate(() => window.__openRebellionInterfaceFleetLoadSetup);
}

async function fleetLoad(page, predicate) {
  await page.waitForFunction(
    (source) => {
      const last = (window.__openRebellionInterfaceFleetLoads || []).at(-1);
      return last && new Function("o", `return (${source})(o);`)(last);
    },
    predicate.toString(),
    { timeout: 10_000, polling: 100 },
  );
  return page.evaluate(() => window.__openRebellionInterfaceFleetLoads.at(-1));
}

// The sector window's fleet icon opens the Fleet window (fleet-window.mjs).
async function openFleetWindow(page) {
  const setup = await fleetLoadSetup(page);
  await doubleClick(page, point(setup.icon));
  return fleetLoad(page, (o) => o.window_open && o.fleet_entry);
}

// Each case reaches one family's object and returns the point to right-click.
// The fixture's 235-pixel Defenses window sits 5 pixels in from the galaxy
// view's top-right corner; the agent's cell is centred 42 by 116 into it
// (mission-dialog.mjs).
// The Fleet Status title, the first four labels and the first value (font
// 5 and font 10, set by theme::paint_gdi_text) against A0 06-fleet: each
// ink box within a pixel of A0's, relative to the window.
function fleetStatusText(port, rect) {
  const a0 = PNG.sync.read(fs.readFileSync(path.join(a0Dir, "06-fleet.png")));
  const ink = (png, ox, oy, [x0, y0, x1, y1]) => {
    let box = null;
    for (let y = y0; y < y1; y += 1) {
      for (let x = x0; x < x1; x += 1) {
        const i = ((oy + y) * png.width + ox + x) * 4;
        if (Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) <= 230) continue;
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
      }
    }
    return box;
  };
  const checks = {};
  for (const [name, region] of [["title", [15, 18, 226, 36]], ["status", [16, 47, 118, 61]],
    ["admiral", [16, 61, 118, 75]], ["general", [16, 75, 118, 89]], ["commander", [16, 89, 118, 103]],
    ["status-value", [119, 47, 226, 61]]]) {
    const a = ink(a0, 170, 81, region);
    const p = ink(port, rect.x, rect.y, region);
    checks[name] = { a0: a, port: p };
    assert.ok(a && p && a.every((value, k) => Math.abs(value - p[k]) <= 1), `${name}: ${JSON.stringify(checks[name])}`);
  }
  return checks;
}

// The facility's rows against A0 (labels at x 16, values at x 119, rows
// 14 apart from y 47; the processing rate's label wraps to two lines):
// each ink box within 1 pixel, right edges within 3 (the accepted text
// deviation). The A0 capture's Training Facility stands at Coruscant, the
// fixture's at its own primary system, so the Location value is skipped.
function facilityStatusText(port, rect, ref) {
  const file = path.join(root, ".artifacts/a0-vm/status", ref.file);
  if (!fs.existsSync(file)) return null;
  const a0 = PNG.sync.read(fs.readFileSync(file));
  const ink = (png, ox, oy, [x0, y0, x1, y1]) => {
    let box = null;
    for (let y = y0; y < y1; y += 1) {
      for (let x = x0; x < x1; x += 1) {
        const i = ((oy + y) * png.width + ox + x) * 4;
        if (Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) <= 200) continue;
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
      }
    }
    return box;
  };
  const checks = {};
  const regions = ref.labels
    ? [["title", [15, 18, 226, 36]],
      ...Array.from({ length: ref.labels }, (_, k) => [`label-${k}`, [16, 47 + 14 * k, 118, 61 + 14 * k]])
        .filter((_, k) => !(ref.skip ?? []).includes(k)),
      ...Object.entries(ref.values ?? {}).map(([name, k]) => [name, [119, 47 + 14 * k, 210, 61 + 14 * k]])]
    : [["title", [15, 18, 226, 36]], ["location", [16, 47, 118, 61]],
      ["status", [16, 61, 118, 75]], ["status-value", [119, 61, 226, 75]], ["maintenance", [16, 75, 118, 89]],
      ["processing", [16, 89, 118, 103]], ["rate", [16, 103, 118, 117]], ["bombardment", [16, 117, 118, 131]]];
  for (const [name, region] of regions) {
    const a = ink(a0, ref.x, ref.y, region);
    const p = ink(port, rect.x, rect.y, region);
    checks[name] = { a0: a, port: p };
    assert.ok(a && p && Math.abs(a[0] - p[0]) <= (ref.leftSlack ?? 1) && [1, 3].every((k) => Math.abs(a[k] - p[k]) <= 1)
      && Math.abs(a[2] - p[2]) <= 3,
      `${name}: ${JSON.stringify(checks[name])}`);
  }
  return checks;
}

const cases = [
  {
    name: "character",
    code: MISSION_TARGETING,
    family: "Character",
    closes: true,
    // A0 status/14-char-menu (Veers at Coruscant): a 124 by 142 menu,
    // Move, Confirmed Move, Mission, Command, Encyclopedia, Status, Retire.
    // Command's state follows the character (Veers can command; the
    // fixture's agent cannot).
    a0Menu: { width: 124, height: 142,
      rows: ["Move", "Confirmed Move", "Mission", "Command", "Encyclopedia", "Status", "Retire"] },
    target: async (page, faction) => ({ x: faction.galaxy.x + faction.galaxy.width - 235 - 5 + 42, y: faction.galaxy.y + 5 + 116 }),
  },
  {
    name: "force-character",
    code: FORCE_STATUS,
    family: "Character",
    // FUN_004486f0's Force Ranking by +0x8c: A0 personnel-states/19 (Luke,
    // "Trainee") and status/33-emperor ("Jedi Master").
    forceRanking: { alliance: "Trainee", empire: "Jedi Master" },
    target: async (page, faction) => ({ x: faction.galaxy.x + faction.galaxy.width - 235 - 5 + 42, y: faction.galaxy.y + 5 + 116 }),
  },
  {
    name: "regiment",
    // A0 status/11-troop-menu: 104 by 102.
    a0Menu: { width: 104, height: 102, rows: ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"] },
    // A0 troopstatus/03 (the Alliance's Army Regiment at Mon Calamari):
    // "Trooper Regiment Status" over Attached, Status, Maintenance Cost,
    // Attack Strength, Defense Strength, Bombardment Value, Detection Value.
    a0: { file: "../troopstatus/03-status.png", x: 108, y: 79, labels: 7, side: "alliance" },
    code: FLEET_LOAD,
    family: "Troop",
    target: async (page) => point((await fleetLoadSetup(page)).troop_item),
  },
  {
    name: "fleet",
    code: FLEET_LOAD,
    family: "Fleet",
    target: async (page) => point((await openFleetWindow(page)).fleet_entry),
  },
  {
    name: "capital-ship",
    // A0 status/02-menu (an enemy ship's menu, the same rows): 104 by 142.
    a0Menu: { width: 104, height: 142,
      rows: ["Move", "Confirmed Move", "Create Fleet", "Rename", "Encyclopedia", "Status", "Scrap"] },
    code: FLEET_LOAD,
    family: "Ship",
    // A0 status/03-ship (Corellian Corvette 1, undamaged): the first page's
    // fourteen labels, Class to Hull Value, and Ship Damaged's "No".
    // The fixture's ship is another class, damaged, so only the labels are
    // compared; Liberation Sans's "T" stands up to 3 pixels right of GDI
    // Arial's (the accepted text deviation).
    a0: { file: "03-ship.png", x: 170, y: 81, labels: 14, leftSlack: 3 },
    target: async (page) => {
      const opened = await openFleetWindow(page);
      await click(page, point(opened.fleet_entry));
      return point((await fleetLoad(page, (o) => o.selected === "fleet" && o.first_item)).first_item);
    },
  },
  // A squadron on the planet (the Defenses window's squadron page, as
  // fleet-window.mjs's squadron cases reach it): A0 status/21-fighter (a
  // TIE Fighter squadron at Coruscant): Attached .. Bombardment Value, the
  // Weapons Rating heading and its rows, "Maximum Shield Strength" and
  // "Sub-Light Engine Rating" on two lines each.
  {
    name: "fighter",
    // A0 status/20-fighter-menu: 104 by 102.
    a0Menu: { width: 104, height: 102, rows: ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"] },
    code: SQUADRON_LOAD,
    family: "PlanetFighter",
    // A0's pointer covers rows 9 and 10 (Detection Rating, Bombardment
    // Value); row 12 is " Laser Rating:", one space in.
    a0: { file: "21-fighter.png", x: 170, y: 81, labels: 13, skip: [9, 10] },
    target: async (page) => point((await fleetLoadSetup(page)).troop_item),
  },
  // A special force's Status (FUN_00445780) from the Defenses window's
  // personnel page, where the Recon fixture puts its recon team first:
  // A0 sfstatus/03 (the Alliance's Infiltrators at Yavin, en route):
  // "Spec Forces Status" over Attached, Status, ETA Destination,
  // Maintenance Cost and the four ratings. The fixture's team is not en
  // route, so the title and the first two labels are compared.
  {
    name: "special-force",
    code: RECON_TARGETING,
    family: "SpecialForce",
    a0: { file: "../sfstatus/03-sf-status.png", x: 108, y: 79, labels: 2, side: "alliance" },
    // A0 sfstatus/02: 104 by 122, Move, Confirmed Move, Mission,
    // Encyclopedia, Status, Retire (grey there: the unit is en route).
    a0Menu: { width: 104, height: 122,
      rows: ["Move", "Confirmed Move", "Mission", "Encyclopedia", "Status", "Retire"] },
    target: async (page, faction) => ({ x: faction.galaxy.x + faction.galaxy.width - 235 - 5 + 42, y: faction.galaxy.y + 5 + 116 }),
  },
  // The en route special force (OBJ-10 C004): A0 sfstatus/03, the
  // Alliance's Infiltrators travelling to Yavin, listed on Yavin's Defenses
  // window. FUN_00445780's eight rows, Status "Enroute" with the ETA row;
  // FUN_0042c3b0 draws STRATEGY 11520 (the en route mark) under the
  // Infiltrators' picture. The title and all eight labels are compared with
  // A0, and the picture frame's interior in every pixel.
  {
    name: "special-force-en-route",
    code: SPECIAL_FORCE_ENROUTE,
    family: "SpecialForce",
    a0: { file: "../sfstatus/03-sf-status.png", x: 108, y: 79, labels: 8, side: "alliance" },
    a0Picture: { file: "../sfstatus/03-sf-status.png", x: 108, y: 79, side: "alliance" },
    rows: [["Attached: "], ["Status:", "Enroute"], ["ETA Destination:", "Day 249"], ["Maintenance Cost:"],
      ["Diplomacy Rating:"], ["Espionage Rating:"], ["Combat Rating:"], ["Leadership Rating:"]],
    marks: [["Strategy", 11520]],
    // Its cell on the Defenses window: A0 sfstatus/01 lists the travelling
    // Infiltrators third on Yavin's personnel page, the mini at (452, 167)
    // over the en route mark STRATEGY 11501 (hyperspace through the
    // ports), not the backdrop 11500 alone (FUN_0042c3b0, +0x50 bit 4).
    // The 61 by 25 mini is found in the port's window and compared in
    // every pixel but 11501's key pixel (0, 24), where the window's own
    // background shows.
    a0Cell: { file: "../sfstatus/01-yavin.png", x: 452, y: 167, width: 61, height: 25, skip: [[0, 24]], side: "alliance" },
    a0Menu: { width: 104, height: 122,
      rows: ["Move", "Confirmed Move", "Mission", "Encyclopedia", "Status", "Retire"] },
    target: async (page, faction) => ({ x: faction.galaxy.x + faction.galaxy.width - 235 - 5 + 42, y: faction.galaxy.y + 5 + 116 }),
  },
  // The en route regiment (OBJ-10 C004): A0 2026-11-11-regiment-en-route/
  // 03, 04 (an Alliance game, day 46..47: the Alliance Army Regiment moving
  // from Yavin, listed first on Averam's Trooper Regiments page). Its menu
  // greys Move, Confirmed Move and Scrap; its Status (FUN_00445280) reads
  // Enroute with the ETA row; FUN_0042c3b0 draws the backdrop 11514, then
  // the class's own GOKRES en route mark (portrait + 0x1000), under the
  // picture. Title, labels and the picture's interior against A0.
  {
    name: "regiment-en-route",
    code: REGIMENT_ENROUTE,
    family: "Troop",
    a0: { file: `${regimentA0}/04-first-regiment-status.png`, x: 108, y: 79, labels: 8, side: "alliance" },
    a0Picture: { file: `${regimentA0}/04-first-regiment-status.png`, x: 108, y: 79, side: "alliance" },
    rows: [["Attached: "], ["Status:", "Enroute"], ["ETA Destination:"], ["Maintenance Cost:"], ["Attack Strength:"],
      ["Defense Strength:"], ["Bombardment Value:"], ["Detection Value:"]],
    marks: { alliance: [["Strategy", 11514], ["Gokres", 5185]], empire: [["Strategy", 11514], ["Gokres", 5250]] },
    a0Menu: { width: 104, height: 102, rows: ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"],
      states: [false, false, true, true, false] },
    target: async (page, faction) => ({ x: faction.galaxy.x + faction.galaxy.width - 235 - 5 + 42, y: faction.galaxy.y + 5 + 116 }),
  },
  // The en route squadron: FUN_00556390 puts a squadron sent by Move in its
  // destination at once, in hyperspace (+0x50 bit 4), so the destination's
  // Fighter Squadrons page lists it (FUN_005039d0), as A0
  // 2026-11-11-regiment-en-route/00 shows for a regiment. Its cell is the
  // mini over its own en route mark (FUN_0042c3b0, the mini + 0x1000);
  // its menu greys Move, Confirmed Move and Scrap as the en route
  // regiment's does (A0 /03); its Status (FUN_004444c0) is attached to the
  // destination with FUN_004fd2b0's ETA and the portrait's own en route mark
  // (portrait + 0x1000). No A0 capture of a travelling squadron.
  {
    name: "squadron-en-route",
    code: SQUADRON_ENROUTE,
    family: "TravellingFighter",
    rows: [["Attached: "], ["ETA Destination:"], ["Maintenance Cost:"], ["Squadron Size:"], ["Hyperdrive Rating:"],
      ["Maximum Shield Strength:"], ["Sub-Light Engine Rating:"], ["Maneuverability:"], ["Detection Rating:"],
      ["Bombardment Value:"], ["Weapons Rating:"], [" Laser Rating:"], [" Ion Cannon:"], [" Torpedoes:"]],
    // GOKRES 17984 (A-wing) and 18048 (TIE Fighter) less 0x4000, plus 0x1000.
    marks: { alliance: [["Gokres", 5696]], empire: [["Gokres", 5760]] },
    cellComposite: { alliance: 17984, empire: 18048 },
    // The second squadron travels to the second system, which holds
    // nothing else the defenses icon counts (FUN_0045ce80): its icon shows,
    // the side's STRATEGY art (FUN_0045ca80), in every unkeyed pixel.
    defensesIcon: { alliance: 10773, empire: 10781 },
    a0Menu: { width: 104, height: 102, rows: ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"],
      states: [false, false, true, true, false] },
    target: async (page, faction) => ({ x: faction.galaxy.x + faction.galaxy.width - 235 - 5 + 42, y: faction.galaxy.y + 5 + 116 }),
  },
  // Units on their way into a fleet (FUN_00556390 into the fleet at once;
  // regiment-unload.md): its Fighters, Troops and Personnel tabs light and
  // list them although its hold is empty, each over its en route mark. The
  // squadron's menu greys Move, Confirmed Move and Scrap; its Status is
  // attached to the fleet with the ETA. No A0 capture.
  {
    name: "fleet-squadron-en-route",
    code: FLEET_ARRIVALS,
    family: "TravellingFighter",
    rows: [["Attached: "], ["ETA Destination:"], ["Maintenance Cost:"], ["Squadron Size:"], ["Hyperdrive Rating:"],
      ["Maximum Shield Strength:"], ["Sub-Light Engine Rating:"], ["Maneuverability:"], ["Detection Rating:"],
      ["Bombardment Value:"], ["Weapons Rating:"], [" Laser Rating:"], [" Ion Cannon:"], [" Torpedoes:"]],
    marks: { alliance: [["Gokres", 5696]], empire: [["Gokres", 5760]] },
    cellComposite: { alliance: 17984, empire: 18048 },
    a0Menu: { width: 104, height: 102, rows: ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"],
      states: [false, false, true, true, false] },
    target: async (page) => {
      const opened = await openFleetWindow(page);
      await click(page, point(opened.fleet_entry));
      const fleet = await fleetLoad(page, (o) => o.selected === "fleet" && o.troops_tab && o.fighters_tab);
      assert.deepEqual(fleet.enabled, [true, true, true, true], JSON.stringify(fleet));
      await click(page, point(fleet.troops_tab));
      const troops = await fleetLoad(page, (o) => o.tab === "Troops" && o.items.length > 0);
      assert.equal(troops.items.length, 1, JSON.stringify(troops));
      await click(page, { x: Math.round(fleet.troops_tab[0] + 32), y: Math.round(fleet.troops_tab[1]) });
      const personnel = await fleetLoad(page, (o) => o.tab === "Personnel" && o.items.length > 0);
      // The character and the special force on their way in.
      assert.equal(personnel.items.length, 2, JSON.stringify(personnel));
      await click(page, point(fleet.fighters_tab));
      const fighters = await fleetLoad(page, (o) => o.tab === "Fighters" && o.items.length > 0 && o.first_item);
      assert.equal(fighters.items.length, 1, JSON.stringify(fighters));
      // FUN_004a6be0 sums the ships' slots +0x23c / +0x240, which count
      // their children by walk mode 3 (CapShip_vt23c, CapShip_vt240:
      // FUN_005039d0 / FUN_00504c40, FUN_00513180), the list's own walk:
      // the travellers count as aboard.
      assert.equal(troops.counts?.[0], troops.items.length, JSON.stringify(troops));
      assert.equal(fighters.counts?.[0], fighters.items.length, JSON.stringify(fighters));
      return point(fighters.first_item);
    },
  },
  // The receiving ship's Capital Ship Status (FUN_00446fd0): Embarked
  // prints the ship's slots +0x23c / +0x240 (CapShip_vt23c FUN_00500fe0,
  // CapShip_vt240 FUN_00501040) and FUN_004f25c0(ship, 3)'s characters,
  // all walk mode 3 (FUN_00513180), so the squadron, regiment and
  // character on their way in count where they are seated (FUN_00552300).
  // The fixture's fleet is one ship, so its Embarked equals the Fleet
  // window's Fighters and Troops counters and Personnel list's
  // characters: FUN_004f25c0 walks families 0x30..0x3c (FUN_005131d0
  // keeps lo <= family < hi), so the special force on its way in, which
  // the list shows, is not counted. No A0 capture.
  {
    name: "fleet-ship-en-route",
    code: FLEET_ARRIVALS,
    family: "Ship",
    embarked: () => fleetShipEmbarked,
    target: async (page) => {
      fleetShipEmbarked = null;
      const { counts, personnel, back } = await fleetEmbarked(page);
      fleetShipEmbarked = [...counts, String(personnel.filter((item) => !isSpecialForce(item)).length)];
      return point(back.first_item);
    },
  },
  // The receiving fleet's Fleet Status (FUN_00449200): Embarked sums its
  // ships' slots, and Personnel each ship's FUN_00536da0(ship, 3) count,
  // families 0x30..0x40, so the special force on its way in counts with
  // the character. It equals the Fleet window's counters and its whole
  // Personnel list. No A0 capture.
  {
    name: "fleet-status-en-route",
    code: FLEET_ARRIVALS,
    family: "Fleet",
    embarked: () => fleetShipEmbarked,
    target: async (page) => {
      fleetShipEmbarked = null;
      const { counts, personnel, back } = await fleetEmbarked(page);
      fleetShipEmbarked = [...counts, String(personnel.length)];
      return point(back.fleet_entry);
    },
  },
  // A character on its way into the fleet (the fixture's Move onto it,
  // FUN_00556390 into the target container +0x74): the Personnel tab lists
  // it, and its Status (FUN_004486f0) names the fleet under Attached, then
  // Enroute and the ETA (FUN_004fd2b0). No A0 capture.
  {
    name: "fleet-character-en-route",
    code: FLEET_ARRIVALS,
    family: "Character",
    rows: [["Commanding:"], ["Attached: ", "Fleet 1"], ["Status:", "Enroute"], ["ETA Destination:", "Day 99"],
      ["Force Ranking:"], ["Diplomacy Rating:"], ["Espionage Rating:"], ["Combat Rating:"], ["Leadership Rating:"],
      ["R&D Capabilities"], [" Ship Design"], [" Troop Training"], [" Facility Design"], ["Possible Command Ranks"],
      ["Admiral:"], ["General:"], ["Commander:"]],
    target: async (page) => {
      const opened = await openFleetWindow(page);
      await click(page, point(opened.fleet_entry));
      const fleet = await fleetLoad(page, (o) => o.selected === "fleet" && o.troops_tab);
      await click(page, { x: Math.round(fleet.troops_tab[0] + 32), y: Math.round(fleet.troops_tab[1]) });
      const personnel = await fleetLoad(page, (o) => o.tab === "Personnel" && o.items.length > 0 && o.first_item);
      // The character, then the special force on the same Move.
      assert.equal(personnel.items.length, 2, JSON.stringify(personnel));
      assert.ok(!isSpecialForce(personnel.items[0]) && isSpecialForce(personnel.items[1]), JSON.stringify(personnel));
      return point(personnel.first_item);
    },
  },
  // The regiment in training, the page's second cell: A0 02 reads
  // "Training" with no ETA row, the picture over 11514 and the unfinished
  // mark 11571 (11573 for side 2). A0 01's menu offers Move and Confirmed
  // Move, which the port greys (see the 2026-11-11 evidence), so only the
  // rows are compared.
  {
    name: "regiment-training",
    code: REGIMENT_ENROUTE,
    family: "PendingTroop",
    a0: { file: `${regimentA0}/02-training-regiment-status.png`, x: 108, y: 79, labels: 7, side: "alliance" },
    a0Picture: { file: `${regimentA0}/02-training-regiment-status.png`, x: 108, y: 79, side: "alliance" },
    rows: [["Attached: "], ["Status:", "Training"], ["Maintenance Cost:"], ["Attack Strength:"],
      ["Defense Strength:"], ["Bombardment Value:"], ["Detection Value:"]],
    marks: { alliance: [["Strategy", 11514], ["Strategy", 11571]], empire: [["Strategy", 11514], ["Strategy", 11573]] },
    a0Menu: { width: 104, height: 102, rows: ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"] },
    target: async (page, faction) => ({ x: faction.galaxy.x + faction.galaxy.width - 235 - 5 + 42 + 70, y: faction.galaxy.y + 5 + 116 }),
  },
  // A KDY-150's Status from the Defenses window's Planetary Batteries
  // page (A0 status/22, 23 at Coruscant): Location, Status "Active",
  // Maintenance Cost, Weapons Rating, Shield Strength, Bombardment Defense
  // Strength (two lines).
  {
    name: "battery",
    // A0 status/22-battery-menu: 90 by 62.
    a0Menu: { width: 90, height: 62, rows: ["Encyclopedia", "Status", "Scrap"] },
    code: BATTERY_STATUS,
    family: "DefenseFacility",
    a0: { file: "23-battery.png", x: 170, y: 81, labels: 7, values: { "status-value": 1 }, leftSlack: 1 },
    target: async (page, faction) => ({ x: faction.galaxy.x + faction.galaxy.width - 235 - 5 + 42, y: faction.galaxy.y + 5 + 116 }),
  },
  // A training facility's Status (FUN_005399e0's Status on a page cell):
  // A0 status/29 (Coruscant's Training Facility at (170, 81)): Location,
  // Status "Active", Maintenance Cost, Standard Processing Rate (two
  // lines), Bombardment Value.
  {
    name: "manufacturing-facility",
    // A0 status/28-fac-menu: 90 by 62.
    a0Menu: { width: 90, height: 62, rows: ["Encyclopedia", "Status", "Scrap"] },
    code: BUILD_SELECTION,
    family: "ManufacturingFacility",
    a0: { file: "29-manufacturing.png", x: 170, y: 81 },
    target: async (page) => {
      await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.training_tab, null, { timeout: 10_000 });
      const [tx, ty, tw, th] = (await page.evaluate(() => window.__openRebellionInterfaceProduction)).training_tab;
      await click(page, { x: tx + tw / 2, y: ty + th / 2 });
      await page.mouse.move(2, 2);
      await frames(page);
      await page.waitForTimeout(300);
      const [cx, cy, cw, ch] = (await page.evaluate(() => window.__openRebellionInterfaceProduction)).first_cell;
      return { x: cx + cw / 2, y: cy + ch / 2 };
    },
  },
  {
    name: "production-manager",
    code: BUILD_SELECTION,
    family: "Producer",
    // A0 menus3/34: the menu for nothing, off the bands.
    offBandMenu: { width: 90, height: 42, rows: [["Encyclopedia", false], ["Status", false]] },
    target: async (page) => {
      await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.bands?.length === 3, null, { timeout: 10_000 });
      const [band] = (await page.evaluate(() => window.__openRebellionInterfaceProduction)).bands;
      return { x: band.left + band.width / 2, y: band.top + band.height / 2 };
    },
  },
];

async function inspect(server, source, faction, testCase, executable) {
  const directory = path.join(runDir, faction.name, testCase.name);
  fs.mkdirSync(directory, { recursive: true });
  const errors = [];
  const consoleLines = [];
  const launchAttempts = [];
  let browser;
  let context;
  let page;
  let result;
  const logged = async (description, pattern, count = 1, timeout = 10_000) => {
    const deadline = Date.now() + timeout;
    while (Date.now() < deadline) {
      const lines = consoleLines.filter((line) => pattern.test(line));
      if (lines.length >= count) return lines[count - 1];
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    throw new Error(`${description}: ${pattern} x${count} not logged; last lines ${JSON.stringify(consoleLines.slice(-6))}`);
  };
  try {
    browser = await launchBrowser(chromium, { executablePath: executable, headless: true, args: browserManifest.launch_arguments, timeout: 30_000 }, launchAttempts);
    context = await browser.newContext({ viewport: { width: 640, height: 480 }, deviceScaleFactor: 1, locale: "en-US", timezoneId: "America/New_York", colorScheme: "dark", reducedMotion: "reduce", serviceWorkers: "block" });
    page = await context.newPage();
    page.on("pageerror", (error) => errors.push(`pageerror:${error.stack || error.message}`));
    page.on("console", (message) => {
      consoleLines.push(message.text());
      if (message.type() === "error" || /\[bmp_cache\] asset unavailable/i.test(message.text())) errors.push(`console:${message.type()}:${message.text()}`);
    });
    const fixtureCode = testCase.code | (faction.byte << 8);
    await page.goto(`http://127.0.0.1:${server.address().port}/?fixture-code=${fixtureCode}`, { waitUntil: "load", timeout: 30_000 });
    await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status, null, { timeout: 30_000 });
    const ready = await page.evaluate(() => window.__openRebellionInterfaceReady);
    assert.equal(ready.status, "ready", JSON.stringify(ready));
    await page.evaluate(() => document.fonts.ready);

    const target = await testCase.target(page, faction);
    // The listed cell's mini and marks against A0, at the best offset near
    // the target (the cell's top left is up to 70 by 50 from it).
    let cellCheck = null;
    // Or the cell composed from the owned GOKRES bitmaps (testCase.cellComposite):
    // the en route mark (the mini + 0x1000), then the mini keyed over it;
    // pixels where both are the blue key show the window and are skipped.
    let cellReference = null;
    if (testCase.a0Cell && faction.name === testCase.a0Cell.side) {
      const cell = testCase.a0Cell;
      const a0 = PNG.sync.read(fs.readFileSync(path.join(root, ".artifacts/a0-vm/status", cell.file)));
      const skip = new Set(cell.skip.map(([x, y]) => `${x},${y}`));
      cellReference = { cell, skip, at: (x, y) => ((cell.y + y) * a0.width + cell.x + x) * 4, data: a0.data, label: "A0" };
    } else if (testCase.cellComposite) {
      const mini = testCase.cellComposite[faction.name];
      const gokres = (id) => {
        const bytes = fs.readFileSync(path.join(gokresDirectory(), `${id}.bmp`));
        return decodeIndexedBmp(bytes, bytes.readInt32LE(18), Math.abs(bytes.readInt32LE(22)));
      };
      const face = gokres(mini);
      const mark = gokres(mini + 0x1000);
      // Both at the cell's mini origin; a fighter's 66-wide mini covers
      // the 61-wide mark.
      const width = Math.max(face.width, mark.width);
      const height = Math.max(face.height, mark.height);
      const opaque = (image, x, y) => {
        if (x >= image.width || y >= image.height) return null;
        const i = (y * image.width + x) * 4;
        const blue = image.data[i] === 0 && image.data[i + 1] === 0 && image.data[i + 2] === 255;
        return blue ? null : image.data.subarray(i, i + 4);
      };
      const composite = new PNG({ width, height });
      const skip = new Set();
      for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) {
          const top = opaque(face, x, y) ?? opaque(mark, x, y);
          if (top) composite.data.set(top, (y * width + x) * 4);
          else skip.add(`${x},${y}`);
        }
      }
      fs.writeFileSync(path.join(directory, "cell-expected.png"), PNG.sync.write(composite));
      cellReference = { cell: { width, height }, skip, at: (x, y) => (y * width + x) * 4, data: composite.data, label: `GOKRES ${mini + 0x1000} under ${mini}` };
    }
    // The defenses icon of the quadrant setup's second system against the
    // side's STRATEGY art, its blue key skipped.
    let iconCheck = null;
    if (testCase.defensesIcon) {
      await page.waitForFunction(() => window.__openRebellionInterfaceQuadrantSetup, null, { timeout: 10_000 });
      const setup = await page.evaluate(() => window.__openRebellionInterfaceQuadrantSetup);
      const entry = setup.quadrants.find((q) => q.system_dat_id === setup.second_dat_id && q.quadrant === "defenses");
      assert.ok(entry, JSON.stringify(setup));
      // The art at the icon's top left, at its own size (paint_native).
      const [left, top] = entry.rect.map(Math.round);
      const id = testCase.defensesIcon[faction.name];
      const bytes = fs.readFileSync(path.join(source, `${id}.bmp`));
      const art = decodeIndexedBmp(bytes, bytes.readInt32LE(18), Math.abs(bytes.readInt32LE(22)));
      const { width, height } = art;
      await page.mouse.move(2, 2);
      await frames(page);
      await page.waitForTimeout(300);
      const shot = await page.screenshot({ animations: "disabled" });
      fs.writeFileSync(path.join(directory, "defenses-icon-screen.png"), shot);
      const screen = PNG.sync.read(shot);
      let checked = 0;
      let differing = 0;
      for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) {
          const i = (y * width + x) * 4;
          if (art.data[i] === 0 && art.data[i + 1] === 0 && art.data[i + 2] === 255) continue;
          checked += 1;
          const j = ((top + y) * screen.width + left + x) * 4;
          if ([0, 1, 2].some((k) => art.data[i + k] !== screen.data[j + k])) differing += 1;
        }
      }
      iconCheck = { id, rect: entry.rect, checked, differing };
      assert.ok(checked > 0 && differing === 0, `the defenses icon differs from STRATEGY ${id}: ${JSON.stringify(iconCheck)}`);
    }
    if (cellReference) {
      const { cell, skip } = cellReference;
      await page.mouse.move(2, 2);
      await frames(page);
      await page.waitForTimeout(300);
      const port = PNG.sync.read(await page.screenshot({ animations: "disabled" }));
      const differing = (ox, oy) => {
        let count = 0;
        for (let y = 0; y < cell.height; y += 1) {
          for (let x = 0; x < cell.width; x += 1) {
            if (skip.has(`${x},${y}`)) continue;
            const i = cellReference.at(x, y);
            const j = ((oy + y) * port.width + ox + x) * 4;
            if ([0, 1, 2].some((k) => cellReference.data[i + k] !== port.data[j + k])) count += 1;
          }
        }
        return count;
      };
      let best = null;
      for (let oy = Math.round(target.y) - 60; oy <= Math.round(target.y); oy += 1) {
        for (let ox = Math.round(target.x) - 75; ox <= Math.round(target.x); ox += 1) {
          const count = differing(ox, oy);
          if (!best || count < best.differing) best = { x: ox, y: oy, differing: count };
        }
      }
      cellCheck = { reference: cellReference.label, pixels: cell.width * cell.height - skip.size, ...best };
      fs.writeFileSync(path.join(directory, "cell-port.png"), PNG.sync.write(crop(PNG.sync.write(port), { x: best.x, y: best.y, width: cell.width, height: cell.height })));
      assert.equal(best.differing, 0, `the listed cell differs from ${cellReference.label}: ${JSON.stringify(cellCheck)}`);
    }
    // A right release on the window off its bands, no band selected, opens
    // the pop-up for nothing (FUN_004534f0 -> FUN_004ac5c0): A0
    // menus3/34-icon-menu-reserved, at Coruscant's window's left edge,
    // shows Encyclopedia and Status, both grey, 90 by 42.
    let offBandMenu = null;
    if (testCase.offBandMenu) {
      const { bands } = await page.evaluate(() => window.__openRebellionInterfaceProduction);
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
      await click(page, { x: Math.round(bands[0].left - 35), y: Math.round(bands[0].top + 20) }, "right");
      await page.waitForTimeout(300);
      await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu", null, { timeout: 10_000 });
      offBandMenu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
      fs.writeFileSync(path.join(directory, "off-band-menu.png"), await page.screenshot({ animations: "disabled" }));
      assert.deepEqual([offBandMenu.width, offBandMenu.height], [testCase.offBandMenu.width, testCase.offBandMenu.height],
        JSON.stringify(offBandMenu));
      assert.deepEqual(offBandMenu.row_states, testCase.offBandMenu.rows, JSON.stringify(offBandMenu));
      await page.keyboard.press("Escape");
      await frames(page);
      await page.waitForTimeout(300);
    }
    const openStatus = () => chooseStatus(page, target);
    const opened = new RegExp(`command=0x103 destination=status_window status=opened object=${testCase.family}[ (].+ rect=(\\d+),(\\d+),(\\d+),(\\d+)`);
    const closed = /destination=status_window status=closed action=(\w+)/;

    const menu = await openStatus();
    if (testCase.a0Menu) {
      assert.deepEqual([menu.width, menu.height], [testCase.a0Menu.width, testCase.a0Menu.height], JSON.stringify(menu));
      assert.deepEqual(menu.row_states.map(([label]) => label), testCase.a0Menu.rows, JSON.stringify(menu));
      if (testCase.a0Menu.states) {
        assert.deepEqual(menu.row_states.map(([, enabled]) => enabled), testCase.a0Menu.states, JSON.stringify(menu));
      }
    }
    const line = await logged("Status opens the window", opened);
    const [, x, y, width, height] = line.match(opened).map(Number);
    const rect = { x, y, width, height };
    assert.deepEqual([width, height], [dimensions.width, dimensions.height]);
    let forceRanking = null;
    if (testCase.forceRanking) {
      await page.waitForFunction(() => window.__openRebellionInterfaceStatusRows?.rows?.length, null, { timeout: 10_000 });
      const { rows } = await page.evaluate(() => window.__openRebellionInterfaceStatusRows);
      forceRanking = rows.find(([label]) => label === "Force Ranking:")?.[1] ?? null;
      assert.equal(forceRanking, testCase.forceRanking[faction.name], JSON.stringify(rows));
    }
    let statusRows = null;
    if (testCase.rows) {
      await page.waitForFunction(() => window.__openRebellionInterfaceStatusRows?.rows?.length, null, { timeout: 10_000 });
      statusRows = (await page.evaluate(() => window.__openRebellionInterfaceStatusRows)).rows;
      assert.deepEqual(statusRows.map(([label]) => label), testCase.rows.map(([label]) => label), JSON.stringify(statusRows));
      testCase.rows.forEach(([, value], k) => {
        if (value !== undefined) assert.equal(statusRows[k][1], value, JSON.stringify(statusRows));
      });
      if (testCase.marks) {
        const { marks } = await page.evaluate(() => window.__openRebellionInterfaceStatusRows);
        const expected = Array.isArray(testCase.marks) ? testCase.marks : testCase.marks[faction.name];
        assert.deepEqual(marks, expected, JSON.stringify(marks));
      }
    }
    // The three rows under Embarked against what the case expects.
    let embarked = null;
    if (testCase.embarked) {
      await page.waitForFunction(() => window.__openRebellionInterfaceStatusRows?.rows?.length, null, { timeout: 10_000 });
      const { rows } = await page.evaluate(() => window.__openRebellionInterfaceStatusRows);
      const start = rows.findIndex(([label]) => label === "Embarked:");
      assert.ok(start >= 0, JSON.stringify(rows));
      embarked = rows.slice(start + 1, start + 4);
      assert.deepEqual(embarked.map(([label]) => label), ["Fighter Squadrons:", "Trooper Regiments:", "Personnel:"], JSON.stringify(rows));
      assert.deepEqual(embarked.map(([, value]) => value), testCase.embarked(), JSON.stringify(rows));
    }
    await page.mouse.move(2, 2);
    await page.waitForTimeout(200);
    await frames(page);
    const first = await page.screenshot({ animations: "disabled" });
    await frames(page);
    const second = await page.screenshot({ animations: "disabled" });
    fs.writeFileSync(path.join(directory, "status-screen.png"), second);
    assert.equal(sha256(PNG.sync.write(crop(first, rect))), sha256(PNG.sync.write(crop(second, rect))), "the window did not stabilize");
    // FUN_00443130: every case's object is the player's, so its background
    // is the player's own-side one.
    const comparison = compare(crop(second, rect), composeExpected(source, faction), directory, "window");
    assert.equal(comparison.different_pixels, 0, JSON.stringify(comparison));
    for (const name of Object.keys(masks)) assert.ok(comparison.changed_in_masks[name] > 0, `${name} shows nothing`);
    assert.equal(comparison.picture_blue_pixels, 0, "the picture's blue matte is not keyed");
    // A capital ship's list held at its end (the down arrow at the
    // window's (214..224, 242..250), RCDATA 10650's bar) against A0
    // status/04-ship-page5: the label column's ink rows, the four Weapons
    // Arc blocks' tail, each band within 1 pixel (the labels are every
    // capital ship's).
    let scrolledEnd = null;
    if (testCase.name === "capital-ship" && faction.name === "empire") {
      const a0File = path.join(root, ".artifacts/a0-vm/status/04-ship-page5.png");
      if (fs.existsSync(a0File)) {
        await page.mouse.move(rect.x + 219, rect.y + 246);
        await page.mouse.down();
        await page.waitForTimeout(4000);
        await page.mouse.up();
        await page.mouse.move(2, 2);
        await page.waitForTimeout(300);
        const end = PNG.sync.read(await page.screenshot({ animations: "disabled" }));
        fs.writeFileSync(path.join(directory, "scrolled-end.png"), PNG.sync.write(end));
        const a0 = PNG.sync.read(fs.readFileSync(a0File));
        const bands = (png, ox, oy) => {
          const rows = [];
          for (let y = 47; y < 251; y += 1) {
            let ink = false;
            for (let x = 16; x < 118 && !ink; x += 1) {
              const i = ((oy + y) * png.width + ox + x) * 4;
              ink = Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) > 200;
            }
            rows.push(ink);
          }
          const out = [];
          rows.forEach((on, k) => {
            if (on && (k === 0 || !rows[k - 1])) out.push([47 + k, 47 + k]);
            if (on) out[out.length - 1][1] = 47 + k;
          });
          return out;
        };
        const a = bands(a0, 170, 81);
        const p = bands(end, rect.x, rect.y);
        scrolledEnd = { a0: a, port: p };
        assert.equal(p.length, a.length, JSON.stringify(scrolledEnd));
        assert.ok(a.every(([t, b], k) => Math.abs(t - p[k][0]) <= 1 && Math.abs(b - p[k][1]) <= 1), JSON.stringify(scrolledEnd));
      }
    }
    const textChecks = a0Dir && faction.name === "alliance" && testCase.name === "fleet"
      ? fleetStatusText(PNG.sync.read(second), rect)
      : testCase.a0 && faction.name === (testCase.a0.side ?? "empire") ? facilityStatusText(PNG.sync.read(second), rect, testCase.a0) : null;

    // The picture frame's interior (masks.picture) against A0, every pixel.
    let pictureCheck = null;
    if (testCase.a0Picture && faction.name === testCase.a0Picture.side) {
      const a0 = PNG.sync.read(fs.readFileSync(path.join(root, ".artifacts/a0-vm/status", testCase.a0Picture.file)));
      const port = PNG.sync.read(second);
      const box = masks.picture;
      let differing = 0;
      for (let y = box.y; y < box.y + box.height; y += 1) {
        for (let x = box.x; x < box.x + box.width; x += 1) {
          const i = ((testCase.a0Picture.y + y) * a0.width + testCase.a0Picture.x + x) * 4;
          const j = ((rect.y + y) * port.width + rect.x + x) * 4;
          if ([0, 1, 2].some((k) => a0.data[i + k] !== port.data[j + k])) differing += 1;
        }
      }
      fs.writeFileSync(path.join(directory, "picture-port.png"), PNG.sync.write(crop(second, { x: rect.x + box.x, y: rect.y + box.y, width: box.width, height: box.height })));
      pictureCheck = { pixels: box.width * box.height, differing };
      assert.equal(differing, 0, `the picture differs from A0: ${JSON.stringify(pictureCheck)}`);
    }

    // FUN_00443020's tips (A0 a0-vm/tooltip/122-st-ency.png,
    // 123-st-close.png): a (255, 251, 240) tip with a (128, 128, 128)
    // border, 19 high, its top left at the pointer plus (0, 21). The
    // Encyclopedia button has one while it is enabled (the character
    // cases open the Encyclopedia through it).
    const tipAt = (png, px, py) => {
      const at = (x, y) => {
        const i = (y * png.width + x) * 4;
        return [png.data[i], png.data[i + 1], png.data[i + 2]];
      };
      const same = (a, b) => a[0] === b[0] && a[1] === b[1] && a[2] === b[2];
      const top = py + 21;
      return same(at(px, top), [128, 128, 128]) && same(at(px, top + 18), [128, 128, 128])
        && !same(at(px, top + 19), [128, 128, 128]) && same(at(px + 2, top + 2), [255, 251, 240]);
    };
    const tips = {};
    for (const [name, button] of [["encyclopedia", buttons.encyclopedia], ["close", buttons.close]]) {
      const px = rect.x + button.x + 16;
      const py = rect.y + button.y + 15;
      await page.mouse.move(px, py, { steps: 4 });
      await page.waitForTimeout(1000);
      const shot = await page.screenshot({ animations: "disabled" });
      fs.writeFileSync(path.join(directory, `tip-${name}.png`), shot);
      tips[name] = tipAt(PNG.sync.read(shot), px, py);
      await page.mouse.move(2, 2, { steps: 4 });
      await frames(page);
    }
    assert.equal(tips.close, true, "Close shows no tip");
    if (testCase.closes) assert.equal(tips.encyclopedia, true, "Encyclopedia shows no tip");

    // The window is modal (A0 a0-vm/modal2/07-gid-click.png): the GID
    // control under it opens nothing and the window stays.
    const gidMenu = /destination=gid_menu status=opened/;
    const gidControl = faction.byte === 1 ? { x: 459, y: 414 } : { x: 537, y: 446 };
    await click(page, gidControl);
    for (let i = 0; i < 3; i += 1) await frames(page);
    assert.equal(consoleLines.filter((text) => gidMenu.test(text)).length, 0, "the GID control opened its menu under the Status window");
    assert.equal(consoleLines.filter((text) => closed.test(text)).length, 0, "the GID click closed the Status window");

    const closes = {};
    // 0x65 closes.
    await click(page, { x: rect.x + buttons.close.x + 16, y: rect.y + buttons.close.y + 15 });
    closes.close = (await logged("Close closes it", closed, 1)).match(closed)[1];
    if (testCase.closes) {
      // Escape closes (port: no key slot is traced).
      await openStatus();
      await logged("Status opens it again", opened, 2);
      await page.keyboard.press("Escape");
      closes.escape = (await logged("Escape closes it", closed, 2)).match(closed)[1];
      // 0x66 closes and opens the Encyclopedia.
      await openStatus();
      await logged("Status opens it a third time", opened, 3);
      await click(page, { x: rect.x + buttons.encyclopedia.x + 16, y: rect.y + buttons.encyclopedia.y + 15 });
      closes.encyclopedia = (await logged("Encyclopedia closes it", closed, 3)).match(closed)[1];
      assert.deepEqual(closes, { close: "close", escape: "close", encyclopedia: "encyclopedia" });
      // The Encyclopedia is modal too (A0 a0-vm/modal2/13-ency-gid.png).
      await page.waitForTimeout(200);
      await click(page, gidControl);
      for (let i = 0; i < 3; i += 1) await frames(page);
      assert.equal(consoleLines.filter((text) => gidMenu.test(text)).length, 0, "the GID control opened its menu under the Encyclopedia");
      await page.waitForTimeout(200);
      fs.writeFileSync(path.join(directory, "encyclopedia-screen.png"), await page.screenshot({ animations: "disabled" }));
    } else {
      assert.deepEqual(closes, { close: "close" });
    }
    assert.deepEqual(errors, [], `browser diagnostics: ${errors.join("; ")}`);

    // The menu's Encyclopedia opens straight at the object's topic
    // (FUN_0045d400; A0 encyclopedia/ctx3-luke: Luke Skywalker's).
    let menuTopic = null;
    if (testCase.forceRanking) {
      // The Close above left the galaxy view with no window open.
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
      await click(page, target, "right");
      await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu", null, { timeout: 10_000 });
      const again = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
      const encyclopediaRow = again.row_states.findIndex(([label]) => label === "Encyclopedia");
      const step = (again.height - 2) / again.rows;
      await click(page, { x: again.left + again.width / 2, y: again.top + 2 + step * (encyclopediaRow + 0.5) });
      await page.waitForFunction(() => window.__openRebellionInterfaceEncyclopediaTopic?.topic, null, { timeout: 10_000 });
      menuTopic = await page.evaluate(() => window.__openRebellionInterfaceEncyclopediaTopic);
      assert.equal(menuTopic.topic, "Mon Mothma", JSON.stringify(menuTopic));
    }
    result = { status: "pass", faction: faction.name, case: testCase.name, fixture_code: fixtureCode, object_menu: menu, off_band_menu: offBandMenu, opened: line, rect, forceRanking, menuTopic, statusRows, embarked, pictureCheck, cellCheck, iconCheck, comparison, closes, tips, textChecks, scrolledEnd, console: consoleLines.slice(-20), launch_attempts: launchAttempts };
  } catch (error) {
    result = { status: "fail", faction: faction.name, case: testCase.name, error: String(error.stack || error), errors, console: consoleLines.slice(-20), launch_attempts: launchAttempts };
  } finally {
    if (page) await page.close().catch(() => {});
    if (context) await context.close().catch(() => {});
    if (browser) await browser.close().catch(() => {});
  }
  return result;
}

async function main() {
  const source = sourceDirectory();
  if (!noBuild) execFileSync("bash", [path.join(root, "scripts/build-interface-test-wasm.sh")], { cwd: root, env: { ...process.env }, stdio: "inherit" });
  for (const required of ["index.html", "gl.js", "open-rebellion-test.wasm", "data/runtime.orpk"]) {
    assert.ok(fs.existsSync(path.join(site, required)), `fixture site is missing ${required}`);
  }
  fs.mkdirSync(runDir, { recursive: true });
  const executable = browserExecutable();
  const server = await startServer();
  const results = [];
  try {
    for (const faction of factions) {
      for (const testCase of cases) results.push(await inspect(server, source, faction, testCase, executable));
    }
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
  const passed = results.every((result) => result.status === "pass");
  const summary = {
    schema_version: 1,
    family: "status-window",
    scope: "test-only Status window (FUN_00442d70) for a character, a regiment, a fleet, a capital ship and a production manager, each opened from its pop-up menu: STRATEGY background and buttons compared exactly; title, list, name and picture masked and checked for content and keying; Close closes each, and Escape and Encyclopedia close the character's; on both sides",
    status: passed ? "pass" : "fail",
    browser_version: browserManifest.version,
    viewport: { width: 640, height: 480, device_scale_factor: 1 },
    window: dimensions,
    masks,
    factions: results,
    wasm_sha256: sha256(fs.readFileSync(path.join(site, "open-rebellion-test.wasm"))),
  };
  fs.writeFileSync(path.join(runDir, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
  if (!passed) throw new Error(JSON.stringify(results.filter((result) => result.status !== "pass"), null, 2));
  process.stdout.write(`${JSON.stringify({ run_dir: runDir, status: summary.status, cases: results.map(({ faction, case: name, rect, comparison, closes }) => ({ faction, case: name, rect, comparison, closes })) }, null, 2)}\n`);
}

await main();
