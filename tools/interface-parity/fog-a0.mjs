// The fog of war against the unedited A0 games of 2026-11-19
// (object_views::setup, viewed_contents::apparent_system,
// ghidra/notes/fog-of-war.md).
//
// A0 2026-11-19-fog-13 is a new Empire game, fog-14 continues it and
// starts a new Alliance game. Each fixture keeps the seeded galaxy as a
// new game leaves it (the browser's seed, so its holders and support
// differ from A0's random game): only what the fog decides is compared.
//
// - The GID on Popular Support (NewGame, code 156): every Rim system shows
//   the unexplored mark 10158 but Yavin (the Empire's level-10 push,
//   FUN_0051b990, and the Alliance's own) and the Alliance's headquarters
//   (its own); every core system shows a coloured mark (the setup push's
//   full core copies, mode 1). Yavin's mark is A0's, every non-key pixel
//   (FUN_0042b330 case 0x10: FUN_00426e70 on the viewer's support).
//   Empire: fog-13 e-02; Alliance: fog-14 a-01.
// - The sector windows (FUN_0045bbb0 reads the viewer's copy): the
//   Empire's Mayagil all bare, white names, no tracks, no marks (fog-13
//   e-11); Sumitra bare but Yavin, red, with its tracks and its mark and
//   icons (e-12); the Alliance's Sumitra likewise, Yavin its own (fog-14
//   a-04).
// - Triton (Mayagil, Rim), the Empire's first fleet sent there: in
//   hyperspace it is bare and its GID point gray (TritonEnroute, fog-14
//   e-05, the GID on e-04); in orbit, a mark, tracks, white name, and a
//   coloured GID point (TritonArrived, e-06, the GID on e-10); after the fleet left and Triton's own values
//   changed, the frozen copy's tracks as on arrival (TritonStale, e-13 and
//   e-28, which show A0's day-44 tracks on days 44 and 133).
//
// Each system is read in a fixed box from its planet's corner
// (sector_window::sector_planet_position over SECTORSD/SYSTEMSD): its
// name's colour, its track pixels, and the saturated pixels round the
// planet's left and bottom (the copied GID mark and the facility icons;
// the incoming-fleet arrow on the right is left out).
//
// Build first: cargo build --target wasm32-unknown-unknown -p rebellion-app
//              --release --features interface-test-fixtures (then the site)
// Run:         node fog-a0.mjs [--out=DIR]
// Env:         CHROMIUM_PATH (default /usr/bin/chromium)
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
import { PNG } from "pngjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const SITE = path.join(ROOT, ".artifacts/interface-parity/site");
const A0_DIR = path.join(ROOT, ".artifacts/a0-vm");
// sha256 first 12 hex, as each folder's README lists them.
const A0_FILES = {
  e02: ["2026-11-19-fog-13/e-02-gid-popular-support-d7.png", "d9021b96bd53"],
  e11: ["2026-11-19-fog-13/e-11-sector-mayagil-rim-d9.png", "73cbeab9d48d"],
  e12: ["2026-11-19-fog-13/e-12-sector-sumitra-yavin-d10.png", "3c48fff4112b"],
  a01: ["2026-11-19-fog-14/a-01-gid-popular-support-default-d4.png", "f08c83ca8d12"],
  a04: ["2026-11-19-fog-14/a-04-sector-sumitra-rim-yavin-own-d4.png", "838d0f730341"],
  enroute: ["2026-11-19-fog-14/e-05-sector-mayagil-and-gid-triton-gray-enroute-d40.png", "ed937647a151"],
  arrived: ["2026-11-19-fog-14/e-06-sector-mayagil-and-gid-triton-arrival-d43.png", "5048b9273d8b"],
  left: ["2026-11-19-fog-14/e-13-sector-mayagil-triton-explored-after-fleet-left-d44.png", "b2ef05cac7f5"],
  stale: ["2026-11-19-fog-14/e-28-sector-mayagil-triton-stale-d133.png", "63ba3e75ac9b"],
  // The GID's Triton point: e-05 and e-06 hide it under the hovered
  // sector's label; e-04 (day 30, en route) and e-10 (day 43, the arrival
  // with the label gone) show it.
  gid_enroute: ["2026-11-19-fog-14/e-04-gid-popular-support-fleet1-enroute-past-half-d30.png", "858a751af2c4"],
  gid_arrived: ["2026-11-19-fog-14/e-10-sector-mayagil-and-gid-after-arrival-clean-d43.png", "2ca9165a6138"],
};
const args = Object.fromEntries(process.argv.slice(2).map((arg) => {
  const [key, value = "true"] = arg.replace(/^--/, "").split("=");
  return [key, value];
}));
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `fog-a0-${new Date().toISOString().replace(/[:.]/g, "-")}`));
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".orpk": "application/octet-stream", ".webm": "video/webm" };
// Fixture codes are the Scenario index plus one.
const NEW_GAME = 156;
const TRITON_ENROUTE = 157;
const TRITON_ARRIVED = 158;
const TRITON_STALE = 159;
const ALLIANCE = 1;
const EMPIRE = 2;
const UNEXPLORED = 10158;

// SYSTEMSD.DAT: 16-byte header, 44-byte records: family +16, name +20,
// sector +24, x +36, y +38. SECTORSD.DAT: 36-byte records: id +0, name
// +20, x +32, y +34.
const names = JSON.parse(await fs.readFile(path.join(ROOT, "data/base/textstra.json"), "utf8"));
const systemsd = await fs.readFile(path.join(ROOT, "data/base/SYSTEMSD.DAT"));
const sectorsd = await fs.readFile(path.join(ROOT, "data/base/SECTORSD.DAT"));
const SECTORS = new Map();
for (let i = 0; i < sectorsd.readUInt32LE(4); i += 1) {
  const record = 16 + i * 36;
  SECTORS.set(sectorsd.readUInt32LE(record), { name: names[sectorsd.readUInt16LE(record + 20)],
    x: sectorsd.readUInt16LE(record + 32), y: sectorsd.readUInt16LE(record + 34) });
}
const SYSTEMS = [];
for (let i = 0; i < systemsd.readUInt32LE(4); i += 1) {
  const record = 16 + i * 44;
  SYSTEMS.push({ name: names[systemsd.readUInt16LE(record + 20)], core: systemsd.readUInt32LE(record + 16) === 0x90,
    sector: SECTORS.get(systemsd.readUInt32LE(record + 24)),
    x: systemsd.readUInt16LE(record + 36), y: systemsd.readUInt16LE(record + 38) });
}
const system = (name) => SYSTEMS.find((value) => value.name === name);

function serve(root) {
  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url, "http://localhost");
    const file = path.join(root, decodeURIComponent(url.pathname === "/" ? "/index.html" : url.pathname));
    try {
      const bytes = await fs.readFile(file);
      response.writeHead(200, { "content-type": MIME[path.extname(file)] ?? "application/octet-stream" });
      response.end(bytes);
    } catch {
      response.writeHead(404);
      response.end();
    }
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}

function bmp(bytes) {
  const offset = bytes.readUInt32LE(10);
  const palette = 14 + bytes.readUInt32LE(14);
  const width = bytes.readInt32LE(18);
  const height = bytes.readInt32LE(22);
  const stride = (width + 3) & ~3;
  return {
    width, height,
    pixel(x, y) {
      const entry = palette + bytes[offset + (height - 1 - y) * stride + x] * 4;
      return [bytes[entry + 2], bytes[entry + 1], bytes[entry]];
    },
  };
}
const marks = new Map();
// A mark's non-key pixels (the key is its first DIB pixel, bottom left).
async function artPixels(id) {
  if (!marks.has(id)) {
    const b = bmp(await fs.readFile(path.join(ROOT, `data/base/ui/strategy-dll/BMP/${id}.bmp`)));
    const key = b.pixel(0, b.height - 1).join();
    const pixels = [];
    for (let y = 0; y < b.height; y += 1) {
      for (let x = 0; x < b.width; x += 1) {
        const p = b.pixel(x, y);
        if (p.join() !== key) pixels.push([x, y, p]);
      }
    }
    marks.set(id, pixels);
  }
  return marks.get(id);
}
const at = (png, x, y) => {
  const i = (y * png.width + x) * 4;
  return [png.data[i], png.data[i + 1], png.data[i + 2]];
};
// The largest STRATEGY mark (10146..10160) whose every non-key pixel shows
// at `corner`.
async function markAt(png, [left, top]) {
  let found = null;
  for (let id = 10146; id <= 10160; id += 1) {
    if (id === 10159) continue;
    const pixels = await artPixels(id);
    if (pixels.every(([x, y, p]) => at(png, left + x, top + y).join() === p.join())
      && (!found || pixels.length > found.pixels)) found = { id, pixels: pixels.length };
  }
  return found?.id ?? null;
}
async function markDiff(port, portCorner, a0, a0Corner, id) {
  const pixels = await artPixels(id);
  return pixels.filter(([x, y]) => at(port, portCorner[0] + x, portCorner[1] + y).join()
    !== at(a0, a0Corner[0] + x, a0Corner[1] + y).join()).length;
}
// FUN_00425d00: the system's 17 by 17 cell at (x * 607) >> 10 and
// (y * 437) >> 10 from the side's galaxy origin.
const corner = (value, [ox, oy]) => [((value.x * 607) >> 10) + ox, ((value.y * 437) >> 10) + oy];

// The sector window's buttons (side switch and close, 28 by 13 at the
// window's (204, 2)), from A0 sector-windows/01-first (the left window at
// (60, 35)); a window's origin is where they show.
const reference = PNG.sync.read(await fs.readFile(path.join(A0_DIR, "sector-windows/01-first.png")));
const BUTTONS = [];
for (let y = 0; y < 13; y += 1) for (let x = 0; x < 28; x += 1) BUTTONS.push([x, y, at(reference, 264 + x, 37 + y).join()]);
function windowOrigins(png) {
  const found = [];
  for (let by = 20; by < 80; by += 1) {
    for (let bx = 60; bx < 592; bx += 1) {
      let bad = 0;
      for (const [x, y, p] of BUTTONS) {
        if (at(png, bx + x, by + y).join() !== p && (bad += 1) > 6) break;
      }
      if (bad <= 6) found.push([bx - 204, by - 2]);
    }
  }
  return found;
}
// sector_window::sector_planet_position: the planet's corner in its window.
function planet(value, [wx, wy]) {
  const scaled = (s, sector, factor, extent) => Math.trunc((s - sector) / 1024 * factor * extent);
  return [wx + scaled(value.x, value.sector.x, 13, 235), wy + scaled(value.y, value.sector.y, 10, 360)];
}
// The status tracks' colours (sector_window::status_tracks).
const TRACK_COLOURS = new Set(["128,128,128", "255,255,255", "0,0,255", "255,255,0", "249,92,15", "255,0,0", "0,255,0"]);
// What a system shows in its window, from its planet's corner (px, py):
// - name: the colour (white, red, green) of most name pixels in the 12
//   rows from py + 51, centred on the planet;
// - tracks: track-colour pixels in rows py + 38 .. py + 50;
// - marks: saturated pixels (each channel under 60 or over 195, not grey)
//   in px - 14 .. px + 24, py - 6 .. py + 40 outside the planet's middle.
function systemShows(png, [px, py]) {
  const count = { white: 0, red: 0, green: 0 };
  for (let y = py + 51; y < py + 63; y += 1) {
    for (let x = px - 30; x < px + 67; x += 1) {
      const [r, g, b] = at(png, x, y);
      if (r > 200 && g > 200 && b > 200) count.white += 1;
      else if (r > 200 && g < 60 && b < 60) count.red += 1;
      else if (g > 200 && r < 60 && b < 60) count.green += 1;
    }
  }
  const name = Object.entries(count).sort((a, b) => b[1] - a[1])[0];
  let tracks = 0;
  for (let y = py + 38; y <= py + 50; y += 1) {
    for (let x = px - 20; x < px + 57; x += 1) if (TRACK_COLOURS.has(at(png, x, y).join())) tracks += 1;
  }
  let saturated = 0;
  for (let y = py - 6; y < py + 40; y += 1) {
    for (let x = px - 14; x < px + 24; x += 1) {
      if (x >= px + 4 && x < px + 33 && y >= py + 4 && y < py + 30) continue;
      const p = at(png, x, y);
      if (p.every((c) => c < 60 || c > 195) && !(p.every((c) => c < 60) || p.every((c) => c > 195))) saturated += 1;
    }
  }
  return { name: name[1] >= 6 ? name[0] : null, tracks, marks: saturated };
}
// Each system of `sector` in its window at `origin`, reduced to whether it
// shows tracks and marks.
function sectorShows(png, sector, origin) {
  return Object.fromEntries(SYSTEMS.filter((value) => value.sector.name === sector).map((value) => {
    const shown = systemShows(png, planet(value, origin));
    return [value.name, { ...shown, has_tracks: shown.tracks >= 20, has_marks: shown.marks >= 6 }];
  }));
}
const brief = (shows) => Object.fromEntries(Object.entries(shows)
  .map(([name, { name: colour, has_tracks, has_marks }]) => [name, [colour, has_tracks, has_marks]]));
// The track rows of `value`'s planet, as a hash of their pixels.
function trackHash(png, value, origin) {
  const [px, py] = planet(value, origin);
  const rows = [];
  for (let y = py + 38; y <= py + 50; y += 1) {
    for (let x = px - 20; x < px + 57; x += 1) {
      const p = at(png, x, y).join();
      rows.push(TRACK_COLOURS.has(p) ? p : "-");
    }
  }
  return createHash("sha256").update(rows.join(";")).digest("hex").slice(0, 16);
}

const nextFrames = (page, count = 2) => page.evaluate((n) => new Promise((resolve) => {
  let left = n;
  const step = () => (--left <= 0 ? resolve() : requestAnimationFrame(step));
  requestAnimationFrame(step);
}), count);
async function stable(page, label) {
  let previous = null;
  for (let attempt = 0; attempt < 20; attempt += 1) {
    await nextFrames(page, 3);
    const bytes = await page.screenshot();
    const hash = createHash("sha256").update(bytes).digest("hex");
    if (hash === previous) {
      await fs.writeFile(path.join(OUT, `${label}.png`), bytes);
      return { png: PNG.sync.read(bytes), sha256: hash };
    }
    previous = hash;
  }
  throw new Error(`${label}: the frame never settled`);
}

await fs.mkdir(OUT, { recursive: true });
const a0 = {};
for (const [key, [file, sha]] of Object.entries(A0_FILES)) {
  const bytes = await fs.readFile(path.join(A0_DIR, file));
  assert.equal(createHash("sha256").update(bytes).digest("hex").slice(0, 12), sha, `A0 ${file}`);
  a0[key] = PNG.sync.read(bytes);
}
const result = { status: "fail" };
let server = null;
let browser = null;
const errors = [];
async function open(code, side, label) {
  const page = await (await browser.newContext({ viewport: { width: 640, height: 480 } })).newPage();
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.goto(`http://127.0.0.1:${server.address().port}/?fixture-code=${code | (side << 8)}`);
  await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status === "ready", null,
    { timeout: 30_000 });
  const ready = await page.evaluate(() => window.__openRebellionInterfaceReady);
  await page.locator("#glcanvas").focus();
  await page.mouse.move(2, 470);
  await page.waitForTimeout(800);
  // The galaxy origin: the port's drawn point for the primary system less
  // half its cell, less FUN_00425d00's scaled point.
  const origin = [Math.round(ready.primary_screen_x - 8.5) - ((ready.primary_system_x * 607) >> 10),
    Math.round(ready.primary_screen_y - 8.5) - ((ready.primary_system_y * 437) >> 10)];
  return { page, ready, origin, label };
}
async function finder(page, name) {
  for (const key of ["F2", `type:${name}`, "Enter"]) {
    if (key.startsWith("type:")) await page.keyboard.type(key.slice(5), { delay: 60 });
    else await page.keyboard.press(key);
    await page.waitForTimeout(700);
  }
  await page.mouse.move(2, 470, { steps: 4 });
  await page.waitForTimeout(600);
}
// The GID's marks of every system, A0's and the port's, by class.
async function gidMarks(a0Png, a0Origin, portPng, portOrigin) {
  const rows = {};
  for (const value of SYSTEMS) {
    rows[value.name] = { core: value.core, a0: await markAt(a0Png, corner(value, a0Origin)),
      port: await markAt(portPng, corner(value, portOrigin)) };
  }
  return rows;
}
const coloured = (id) => id !== null && id !== UNEXPLORED;

try {
  server = await serve(SITE);
  browser = await chromium.launch({
    executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium",
    headless: true,
    args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
  });

  // The Empire's GID (fog-13 e-02). A0's origin is the port's (FUN_00421c70
  // places each side's galaxy view); every system's A0 mark found there
  // confirms it.
  const empire = await open(NEW_GAME, EMPIRE, "empire");
  const empireGid = await stable(empire.page, "empire-gid");
  const empireRows = await gidMarks(a0.e02, empire.origin, empireGid.png, empire.origin);
  const yavin = system("Yavin");
  result.empire_gid = { origin: empire.origin, sha256: empireGid.sha256, rows: empireRows };
  const rimColoured = (rows, side) => Object.entries(rows)
    .filter(([, row]) => !row.core && coloured(row[side])).map(([name]) => name).sort();
  // A small galaxy (A0's and the fixture's) leaves 70 systems out: no
  // mark in either.
  const coreGray = (rows, side) => Object.entries(rows)
    .filter(([, row]) => row.core && row[side] === UNEXPLORED).map(([name]) => name).sort();
  const absent = (rows, side) => Object.entries(rows).filter(([, row]) => row[side] === null)
    .map(([name]) => name).sort();
  result.empire_gid.summary = {
    a0_rim_coloured: rimColoured(empireRows, "a0"), port_rim_coloured: rimColoured(empireRows, "port"),
    a0_core_gray: coreGray(empireRows, "a0"), port_core_gray: coreGray(empireRows, "port"),
  };
  result.empire_gid.yavin = { ...empireRows.Yavin };
  result.empire_gid.summary.absent = absent(empireRows, "a0").length;
  assert.deepEqual(absent(empireRows, "port"), absent(empireRows, "a0"), "the systems without a mark");
  assert.deepEqual(result.empire_gid.summary.a0_rim_coloured, ["Yavin"], "A0 e-02: the Rim coloured");
  assert.deepEqual(result.empire_gid.summary.a0_core_gray, [], "A0 e-02: the core gray");
  assert.deepEqual(result.empire_gid.summary.port_rim_coloured, ["Yavin"], JSON.stringify(result.empire_gid.summary));
  assert.deepEqual(result.empire_gid.summary.port_core_gray, [], JSON.stringify(result.empire_gid.summary));
  assert.equal(empireRows.Yavin.port, empireRows.Yavin.a0, JSON.stringify(empireRows.Yavin));
  result.empire_gid.yavin.different = await markDiff(empireGid.png, corner(yavin, empire.origin), a0.e02,
    corner(yavin, empire.origin), empireRows.Yavin.a0);
  assert.equal(result.empire_gid.yavin.different, 0, JSON.stringify(result.empire_gid.yavin));

  // The Empire's Mayagil (e-11, its right window) and Sumitra (e-12, its
  // right window).
  const a0Right = [365, 40];
  for (const [key, [systemName, sector]] of Object.entries({ e11: ["Triton", "Mayagil"], e12: ["Yavin", "Sumitra"] })) {
    assert.ok(windowOrigins(a0[key]).some(([x, y]) => x === a0Right[0] && y === a0Right[1]), `A0 ${key}: no right window`);
    const before = windowOrigins((await stable(empire.page, `before-${sector}`)).png);
    await finder(empire.page, systemName);
    const shown = await stable(empire.page, `empire-${sector}`);
    const origins = windowOrigins(shown.png);
    const origin = origins.find(([x, y]) => !before.some(([bx, by]) => bx === x && by === y));
    assert.ok(origin, `${sector}: no new window (${JSON.stringify(origins)})`);
    const a0Shows = sectorShows(a0[key], sector, a0Right);
    const portShows = sectorShows(shown.png, sector, origin);
    result[`empire_${sector.toLowerCase()}`] = { origin, sha256: shown.sha256, a0: a0Shows, port: portShows };
    // A0: Yavin alone red with tracks and marks; the rest bare and white.
    for (const [name, row] of Object.entries(a0Shows)) {
      const expected = name === "Yavin" ? ["red", true, true] : ["white", false, false];
      assert.deepEqual([row.name, row.has_tracks, row.has_marks], expected, `A0 ${key} ${name}: ${JSON.stringify(row)}`);
    }
    assert.deepEqual(brief(portShows), brief(a0Shows), `${sector}: ${JSON.stringify(result[`empire_${sector.toLowerCase()}`])}`);
  }

  // The Alliance's GID (fog-14 a-01): the Rim gray but its own Yavin and
  // headquarters (A0's game: Anchoron; the port's seed puts it elsewhere).
  const alliance = await open(NEW_GAME, ALLIANCE, "alliance");
  const allianceGid = await stable(alliance.page, "alliance-gid");
  const allianceRows = await gidMarks(a0.a01, alliance.origin, allianceGid.png, alliance.origin);
  result.alliance_gid = { origin: alliance.origin, sha256: allianceGid.sha256, rows: allianceRows };
  result.alliance_gid.summary = {
    a0_rim_coloured: rimColoured(allianceRows, "a0"), port_rim_coloured: rimColoured(allianceRows, "port"),
    a0_core_gray: coreGray(allianceRows, "a0"), port_core_gray: coreGray(allianceRows, "port"),
  };
  assert.deepEqual(absent(allianceRows, "port"), absent(allianceRows, "a0"), "the systems without a mark");
  assert.deepEqual(result.alliance_gid.summary.a0_rim_coloured, ["Anchoron", "Yavin"], "A0 a-01: the Rim coloured");
  assert.deepEqual(result.alliance_gid.summary.a0_core_gray, [], "A0 a-01: the core gray");
  const portRim = result.alliance_gid.summary.port_rim_coloured;
  assert.ok(portRim.length === 2 && portRim.includes("Yavin"), JSON.stringify(result.alliance_gid.summary));
  assert.deepEqual(result.alliance_gid.summary.port_core_gray, [], JSON.stringify(result.alliance_gid.summary));
  result.alliance_gid.yavin = { ...allianceRows.Yavin };
  assert.equal(allianceRows.Yavin.port, allianceRows.Yavin.a0, JSON.stringify(allianceRows.Yavin));

  // The Alliance's Sumitra (a-04, its right window at (300, 35)): its own
  // Yavin red with tracks and marks, the rest bare.
  {
    const a0Origin = [300, 35];
    assert.ok(windowOrigins(a0.a04).some(([x, y]) => x === a0Origin[0] && y === a0Origin[1]), "A0 a-04: no window");
    await finder(alliance.page, "Yavin");
    const shown = await stable(alliance.page, "alliance-Sumitra");
    const origins = windowOrigins(shown.png);
    assert.equal(origins.length, 1, `alliance Sumitra: ${JSON.stringify(origins)}`);
    const a0Shows = brief(sectorShows(a0.a04, "Sumitra", a0Origin));
    const portShows = brief(sectorShows(shown.png, "Sumitra", origins[0]));
    result.alliance_sumitra = { origin: origins[0], sha256: shown.sha256, a0: a0Shows, port: portShows };
    for (const [name, row] of Object.entries(a0Shows)) {
      assert.deepEqual(row, name === "Yavin" ? ["red", true, true] : ["white", false, false], `A0 a-04 ${name}`);
    }
    assert.deepEqual(portShows, a0Shows, JSON.stringify(result.alliance_sumitra));
  }

  // Triton before, on and after the fleet's visit (fog-14).
  const triton = system("Triton");
  const a0Left = [120, 40];
  result.triton = {};
  const a0Tracks = {};
  for (const [key, code] of [["enroute", TRITON_ENROUTE], ["arrived", TRITON_ARRIVED], ["left", TRITON_STALE],
    ["stale", TRITON_STALE]]) {
    assert.ok(windowOrigins(a0[key]).some(([x, y]) => x === a0Left[0] && y === a0Left[1]), `A0 ${key}: no left window`);
    a0Tracks[key] = trackHash(a0[key], triton, a0Left);
    if (key === "stale") {
      // The same port frame as "left": A0 e-28 is e-13 89 days on.
      result.triton.stale = { ...result.triton.left, a0: brief(sectorShows(a0[key], "Mayagil", a0Left)).Triton,
        a0_tracks: a0Tracks[key] };
      continue;
    }
    const view = await open(code, EMPIRE, key);
    const gid = await stable(view.page, `triton-${key}-gid`);
    await finder(view.page, "Triton");
    const shown = await stable(view.page, `triton-${key}`);
    const origins = windowOrigins(shown.png);
    assert.equal(origins.length, 1, `${key}: ${JSON.stringify(origins)}`);
    const a0Shows = sectorShows(a0[key], "Mayagil", a0Left);
    const portShows = sectorShows(shown.png, "Mayagil", origins[0]);
    // The rest of Mayagil: the stale fixture's fleet stands at Kabal, as in
    // A0 e-28 (e-13's was still on its way there).
    const othersA0 = key === "left" ? sectorShows(a0.stale, "Mayagil", a0Left) : a0Shows;
    result.triton[key] = {
      sha256: shown.sha256, origin: origins[0], a0: brief(a0Shows).Triton, port: brief(portShows).Triton,
      a0_tracks: a0Tracks[key], port_tracks: trackHash(shown.png, triton, origins[0]),
      others: { a0: brief(othersA0), port: brief(portShows) },
      gid: { a0: await markAt(a0[key === "enroute" ? "gid_enroute" : "gid_arrived"], corner(triton, view.origin)),
        port: await markAt(gid.png, corner(triton, view.origin)) },
    };
    await view.page.context().close();
  }
  // A0: bare in flight, then a white name with tracks and a mark, kept.
  assert.deepEqual(result.triton.enroute.a0, ["white", false, false], "A0 e-05 Triton");
  for (const key of ["arrived", "left", "stale"]) {
    assert.deepEqual(result.triton[key].a0, ["white", true, true], `A0 ${key} Triton`);
  }
  assert.equal(a0Tracks.left, a0Tracks.arrived, "A0 e-13: the arrival's tracks");
  assert.equal(a0Tracks.stale, a0Tracks.arrived, "A0 e-28: the arrival's tracks");
  for (const key of ["enroute", "arrived", "left"]) {
    const row = result.triton[key];
    assert.deepEqual(row.port, row.a0, `Triton ${key}: ${JSON.stringify(row)}`);
    // The rest of Mayagil stays bare in both.
    // Kabal, where the stale fixture's fleet stands, is live there as in
    // A0 e-28 (white name, mark); its tracks follow each game's seeded
    // values (the browser seed gives it no energy and no raw materials),
    // so they are left out.
    for (const [name, shows] of Object.entries(row.others.port)) {
      if (name === "Triton") continue;
      const [port, a0Row] = name === "Kabal" && key === "left"
        ? [[shows[0], shows[2]], [row.others.a0[name][0], row.others.a0[name][2]]] : [shows, row.others.a0[name]];
      assert.deepEqual(port, a0Row, `${key} ${name}: ${JSON.stringify(row.others)}`);
    }
  }
  // The frozen copy keeps the arrival's tracks (the port's own values).
  assert.equal(result.triton.left.port_tracks, result.triton.arrived.port_tracks, JSON.stringify(result.triton));
  // The GID point: gray in flight, coloured from arrival.
  assert.deepEqual([result.triton.enroute.gid.a0, result.triton.enroute.gid.port], [UNEXPLORED, UNEXPLORED],
    JSON.stringify(result.triton.enroute.gid));
  assert.ok(coloured(result.triton.arrived.gid.a0), JSON.stringify(result.triton.arrived.gid));
  assert.equal(result.triton.arrived.gid.port, result.triton.arrived.gid.a0, JSON.stringify(result.triton.arrived.gid));

  assert.deepEqual(errors, []);
  result.status = "pass";
} catch (error) {
  result.error = String(error.stack || error);
} finally {
  await browser?.close();
  server?.close();
}
const wasm = await fs.readFile(path.join(SITE, "open-rebellion-test.wasm")).catch(() => null);
result.wasm_sha256 = wasm && createHash("sha256").update(wasm).digest("hex");
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: result.status, error: result.error }, null, 2));
if (result.status !== "pass") process.exitCode = 1;
