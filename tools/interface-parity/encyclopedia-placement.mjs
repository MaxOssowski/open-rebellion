// The Encyclopedia's placement in the browser build
// (ghidra/notes/encyclopedia-window.md, "Production routing").
//
// Serves web/, starts a new game as each faction in headless Chromium at
// 640×480, skips the opening briefing and presses F7 (command 0x131), which
// opens the original index. The screenshot is kept for comparison with the
// A0 capture (.artifacts/a0-vm/encyclopedia/), whose window sits at
// (125, 52) for the Empire; captures 281117 and 281123 put the Alliance's
// at (62, 50). The Alliance's run then reopens the index and types "Fle"
// into the Topic box against A0 2026-11-13-all-databases-fleet
// (allDatabasesFleet, OBJ-01-C013).
//
// Build first: scripts/build-wasm.sh
// Run:         node encyclopedia-placement.mjs [--out=DIR] [--faction=alliance|empire]
//              [--window=finders]: F3's Fleet Finder instead, for comparison
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
const WEB = path.join(ROOT, "web");
const args = Object.fromEntries(process.argv.slice(2).map((arg) => {
  const [key, value = "true"] = arg.replace(/^--/, "").split("=");
  return [key, value];
}));
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `encyclopedia-placement-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// The main menu's entries (galaxy-map-visual-check.mjs) and a point on each
// side's agent droid (advisor.rs agent_aperture: Alliance (541,337) 67×116,
// Empire (0,347) 107×133), and a point on the menu's Objectives row: the
// menu ends at the release point, on the droid's right for the Empire and
// on its left for the Alliance. The port's row is at y 307; the A0 capture
// of the Empire's menu, opened at (46, 401), has it at y 311.
// Each side's window origin (A0 and the reference captures), for the list's
// scroll bar: its down arrow at window (380, 292), the list at (36, 137),
// 338 by 160.
const ORIGINS = { alliance: [62, 50], empire: [125, 52] };

// The Personnel Finder's SpecForces view (FUN_004637a0's second view
// button, 0x36 under the first at (423, 147)): its column header strip of
// the four type icons against A0 specforces/01-alliance-grid-day165 (the
// Alliance's finder at (62, 50)), window (256..370, 114..142).
async function specForcesView(page, shot, directory) {
  const [ox, oy] = ORIGINS.alliance;
  await click(page, [ox + 423 + 16, oy + 147 + 54 + 15]);
  await page.mouse.move(2, 2);
  await page.waitForTimeout(500);
  await shot("personnel-finder-specforces");
  const a0File = path.join(ROOT, ".artifacts/a0-vm/specforces/01-alliance-grid-day165.png");
  let a0Bytes;
  try { a0Bytes = await fs.readFile(a0File); } catch { return { a0: null }; }
  const a0 = PNG.sync.read(a0Bytes);
  const port = PNG.sync.read(await fs.readFile(path.join(directory, "personnel-finder-specforces.png")));
  let differing = 0;
  let checked = 0;
  for (let y = 114; y < 142; y += 1) {
    for (let x = 256; x < 370; x += 1) {
      const i = ((oy + y) * port.width + ox + x) * 4;
      checked += 1;
      if (port.data[i] !== a0.data[i] || port.data[i + 1] !== a0.data[i + 1] || port.data[i + 2] !== a0.data[i + 2]) differing += 1;
    }
  }
  return { checked, differing };
}
const FACTIONS = [
  { name: "alliance", menuIndex: 9, droid: [570, 400], objectives: [480, 307], overview: [480, 287] },
  { name: "empire", menuIndex: 8, droid: [46, 401], objectives: [100, 307], overview: [100, 287] },
].filter(({ name }) => !args.faction || args.faction === name);
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".wav": "audio/wav", ".bmp": "image/bmp" };

// The Encyclopedia's text against A0 (--a0=.artifacts/a0-vm/encyclopedia,
// the Empire): each [name, region, ink] box within a pixel of A0's, both
// windows at the Empire's (125, 52). DrawTextA sets them
// (theme::paint_gdi_text): the title and "All Databases" in font 5, the
// rows in the default font (13 ppem), the topic's body DT_WORDBREAK in the
// default font. Index rows A-wing and Abduction are left out: A0's pointer
// and bar touch their regions.
const white = ([r, g, b]) => Math.min(r, g, b) > 200;
const grey = ([r, g, b]) => r > 110 && Math.abs(r - g) < 20 && Math.abs(g - b) < 30 && b > 100;
const INDEX_TEXT = [["title", [200, 66, 480, 82], white], ["topic", [158, 100, 200, 116], white],
  ["all-databases", [164, 172, 300, 188], white], ["ackbar", [164, 230, 400, 249], grey],
  ["adar-tallon", [164, 250, 400, 269], grey], ["advanced-construction-yard", [164, 290, 400, 309], grey]];
const TOPIC_TEXT = [["title", [161, 66, 511, 83], white], ["body-1", [142, 283, 537, 299], white],
  ["body-2", [142, 299, 537, 315], white], ["body-3", [142, 315, 537, 331], white]];
// The Finders' title (font 5, centred in 350), label (font 4) and tab label
// (font 5) against A0's Empire Finders (--a0=.artifacts/a0-vm with
// --window=finders), all at (125, 52).
const FINDER_A0 = { "fleet-finder": "fleet-finder-empire-day167.png",
  "troop-finder": "troop-finder-empire-day167-f4.png", "personnel-finder": "personnel-finder-empire-day167-f5.png" };
const FINDER_TEXT = [["title", [161, 66, 511, 83], white], ["label", [155, 98, 255, 116], white],
  ["tab-label", [163, 169, 300, 187], white]];
async function textAgainstA0(portFile, a0File, regions) {
  const port = PNG.sync.read(await fs.readFile(portFile));
  const a0 = PNG.sync.read(await fs.readFile(path.join(args.a0, a0File)));
  const ink = (png, [x0, y0, x1, y1], pred) => {
    let box = null;
    for (let y = y0; y < y1; y += 1) {
      for (let x = x0; x < x1; x += 1) {
        const i = (y * png.width + x) * 4;
        if (!pred([png.data[i], png.data[i + 1], png.data[i + 2]])) continue;
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
      }
    }
    return box;
  };
  const checks = {};
  for (const [name, region, pred] of regions) {
    const a = ink(a0, region, pred);
    const p = ink(port, region, pred);
    checks[name] = { a0: a, port: p };
    assert.ok(a && p && a.every((value, k) => Math.abs(value - p[k]) <= 1), `${a0File} ${name}: ${JSON.stringify(checks[name])}`);
  }
  return checks;
}

// OBJ-01-C013: F7's All Databases, typing "Fle" into the Topic box, two
// clicks on the list's down arrow and a double click on Fleet, against A0
// 2026-11-13-all-databases-fleet (an Alliance game on day 264, the window
// at (62, 50) as here). Each typed letter runs FUN_00609650 on the box's
// text (FUN_0045d8f0 on 0x408): the best prefix row is chosen, white, and
// scrolled to the list's bottom row; the arrow steps 10 pixels
// (FUN_00607ea0's argument 10); a second click on the row within the
// double-click time presses Topic 0x67 (FUN_0045da70 case 0x65).
// The list (FUN_0045ddc0: window (36, 137), 350 by 160, rows of 20) holds
// the systems of the viewer's galaxy (FUN_00422620); the port's new game
// and A0's game list the same ones in this stretch, so every row is
// compared. Each row's text ink box (grey 0x787878, the chosen one white)
// is within a pixel of A0's (GDI hinting, the accepted text deviation);
// the Fleet entry's artwork (EDATA, window (12, 31), 400 by 200) is
// compared pixel by pixel.
const A0_FLEET = ".artifacts/a0-vm/2026-11-13-all-databases-fleet";
const FLEET_STATES = [
  { label: "typed-F", key: "F", a0: "02-typed-F-chooses-Ftral.png",
    sha256: "05b94e16166a9f78e7ac05a3cc06890c891a69fc06dc238d91acf111031d5c5c", scroll: 0, chosen: 7,
    rows: ["Dorja", "Drall", "Drayson", "Duros", "Emperor Palpatine", "Endor", "Espionage", "F'tral"] },
  { label: "typed-Fl", key: "l", a0: "03-typed-Fl-chooses-Flax.png",
    sha256: "4abeb44bffbd616b46e91c92c607cb418a37afb6f623ec125e0fd2998917fe03", scroll: 0, chosen: 7,
    rows: ["Duros", "Emperor Palpatine", "Endor", "Espionage", "F'tral", "Facility Design Research", "Filve", "Flax"] },
  { label: "typed-Fle", key: "e", a0: "04-typed-Fle-chooses-Fleet.png",
    sha256: "fa79c1ac864a2c9974862ca1cd0c577df0443c7ffa3b18253464a0ae0d1fcbfc", scroll: 0, chosen: 7,
    rows: ["Emperor Palpatine", "Endor", "Espionage", "F'tral", "Facility Design Research", "Filve", "Flax", "Fleet"] },
  { label: "down-once", arrow: true, a0: "05-down-arrow-once-scrolls-half-row.png",
    sha256: "af45e58601210d758c8223075d0e3c95d93333d51a555a03a5e05af1e52b8126", scroll: 10, chosen: 7,
    rows: ["Emperor Palpatine", "Endor", "Espionage", "F'tral", "Facility Design Research", "Filve", "Flax", "Fleet",
      "Fwillsving"] },
  { label: "down-twice", arrow: true, a0: "06-down-arrow-twice-list-endor-to-fwillsving-fleet-white.png",
    sha256: "b0a180607172ba112f3a70b8548e5e15b98122171dea110ab2ca2167e229c4c0", scroll: 0, chosen: 6,
    rows: ["Endor", "Espionage", "F'tral", "Facility Design Research", "Filve", "Flax", "Fleet", "Fwillsving"] },
];
const FLEET_ENTRY = { a0: "07-fleet-entry-double-clicked.png",
  sha256: "b58df141f30f3e1847f13a359890f993ad17fecde98b9d2401e1264cc0f28d21" };
// FLEETSD's class (FUN_004f7d50), the Fleet topic's object.
const FLEET_OBJECT = 0x0800_0004;

function inkBox(png, [x0, y0, x1, y1], pred) {
  let box = null;
  for (let y = Math.max(0, y0); y < y1; y += 1) {
    for (let x = x0; x < x1; x += 1) {
      const i = (y * png.width + x) * 4;
      if (!pred([png.data[i], png.data[i + 1], png.data[i + 2]])) continue;
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
    }
  }
  return box;
}

const near = (a, p) => Boolean(a && p && a.every((value, k) => Math.abs(value - p[k]) <= 1));

async function readA0(file, sha256) {
  const bytes = await fs.readFile(path.join(ROOT, A0_FLEET, file));
  assert.equal(createHash("sha256").update(bytes).digest("hex"), sha256, `${file} is not the recorded A0 capture`);
  return PNG.sync.read(bytes);
}

async function allDatabasesFleet(page, log, shot, directory) {
  const [ox, oy] = ORIGINS.alliance;
  const listX = ox + 36;
  const listY = oy + 137;
  // A row's text: the list's width left of the bar (window x 36..372).
  const rowBox = (top) => [listX + 2, top, listX + 336, top + 20];
  const opened = () => log.filter((line) => line.includes("destination=encyclopedia status=opened_original")).length;
  // The rail's close (0xfb, window (423, 25)), then F7 again: each opening
  // builds a new window at All Databases with nothing chosen (FUN_0045d400).
  const before = opened();
  await click(page, [ox + 423 + 16, oy + 25 + 15]);
  await page.waitForTimeout(300);
  await page.keyboard.down("F7");
  await frames(page);
  await page.keyboard.up("F7");
  await waitFor(page, log, "F7 opens the Encyclopedia again", () => opened() > before, 5_000);
  await page.waitForTimeout(500);
  // The Topic box (A0's press at (325, 104)).
  await click(page, [325, 104]);
  await page.mouse.move(2, 2);
  await page.waitForTimeout(300);
  await shot("all-databases-top");
  const states = [];
  for (const state of FLEET_STATES) {
    if (state.key) {
      await page.keyboard.type(state.key, { delay: 80 });
    } else {
      // The list's down arrow (A0's press at (441, 342)).
      await click(page, [ox + 379, oy + 292]);
    }
    await page.mouse.move(2, 2);
    await page.waitForTimeout(400);
    await shot(`all-databases-${state.label}`);
    const port = PNG.sync.read(await fs.readFile(path.join(directory, `all-databases-${state.label}.png`)));
    const a0 = await readA0(state.a0, state.sha256);
    const rows = state.rows.map((name, index) => {
      const top = listY - state.scroll + 20 * index;
      const pred = index === state.chosen ? white : grey;
      // A row cut by the list's top edge: only its visible part.
      const region = rowBox(Math.max(top, listY));
      region[3] = Math.min(top + 20, listY + 160);
      const a = inkBox(a0, region, pred);
      const p = inkBox(port, region, pred);
      return { name, chosen: index === state.chosen, a0: a, port: p, near: near(a, p) };
    });
    // No other row is white.
    const whiteRows = state.rows.map((_, index) => {
      const top = listY - state.scroll + 20 * index;
      const region = rowBox(Math.max(top, listY));
      region[3] = Math.min(top + 20, listY + 160);
      return Boolean(inkBox(port, region, white));
    });
    // The Topic box's text, white over its black shadow (FUN_00605160):
    // window (142..250, 46..61), inside the box's dotted focus frame.
    const boxRegion = [ox + 142, oy + 46, ox + 250, oy + 61];
    const black = ([r, g, b]) => Math.max(r, g, b) < 32;
    const box = { a0: inkBox(a0, boxRegion, white), port: inkBox(port, boxRegion, white),
      shadowA0: inkBox(a0, boxRegion, black), shadowPort: inkBox(port, boxRegion, black) };
    box.near = near(box.a0, box.port) && near(box.shadowA0, box.shadowPort);
    states.push({ label: state.label, a0: state.a0, rows, whiteRows, box });
  }
  const failures = states.flatMap(({ label, rows, whiteRows, box }) => [
    ...rows.filter((row) => !row.near).map((row) => `${label} ${row.name}: ${JSON.stringify(row)}`),
    ...whiteRows.flatMap((isWhite, index) => (isWhite !== rows[index].chosen
      ? [`${label} row ${index} white=${isWhite}`] : [])),
    ...(box.near ? [] : [`${label} Topic box: ${JSON.stringify(box)}`]),
  ]);
  // The double click on Fleet (A0's at (120, 317), the seventh row).
  const fleetRow = [listX + 22, listY + 20 * 6 + 10];
  // Each press and release in its own frame (egui counts a click per
  // release), both clicks inside egui's 0.3 s.
  await page.mouse.move(...fleetRow);
  await frames(page);
  for (let index = 0; index < 2; index += 1) {
    await page.mouse.down();
    await frames(page, 1);
    await page.mouse.up();
    await frames(page, 1);
  }
  await waitFor(page, log, "the double click opens the Fleet topic",
    () => log.some((line) => line.includes(`[encyclopedia] command=0x67 topic_mode=true object=Some(${FLEET_OBJECT})`)), 5_000);
  await page.mouse.move(2, 470);
  await page.waitForTimeout(800);
  await shot("all-databases-fleet-entry");
  const port = PNG.sync.read(await fs.readFile(path.join(directory, "all-databases-fleet-entry.png")));
  const a0 = await readA0(FLEET_ENTRY.a0, FLEET_ENTRY.sha256);
  // The title (font 5, centred in 350) and the body's one line, the
  // Empire's TOPIC_TEXT regions moved to the Alliance's origin.
  const text = Object.fromEntries([["title", [98, 64, 448, 81]], ["body-1", [79, 281, 474, 297]],
    ["body-2", [79, 297, 474, 313]]].map(([name, region]) => {
    const entry = { a0: inkBox(a0, region, white), port: inkBox(port, region, white) };
    entry.near = (entry.a0 === null && entry.port === null) || near(entry.a0, entry.port);
    return [name, entry];
  }));
  let artDiffering = 0;
  for (let y = oy + 31; y < oy + 231; y += 1) {
    for (let x = ox + 12; x < ox + 412; x += 1) {
      const i = (y * port.width + x) * 4;
      if (port.data[i] !== a0.data[i] || port.data[i + 1] !== a0.data[i + 1] || port.data[i + 2] !== a0.data[i + 2]) artDiffering += 1;
    }
  }
  const entry = { text, art: { region: [ox + 12, oy + 31, 400, 200], checked: 400 * 200, differing: artDiffering } };
  failures.push(...Object.entries(text).filter(([, value]) => !value.near)
    .map(([name, value]) => `Fleet entry ${name}: ${JSON.stringify(value)}`));
  if (artDiffering > 0) failures.push(`Fleet entry art differs from A0 in ${artDiffering} pixels`);
  const result = { states, entry, failures };
  await fs.writeFile(path.join(directory, "all-databases-fleet.json"), `${JSON.stringify(result, null, 2)}\n`);
  assert.deepEqual(failures, [], failures.join("; "));
  return result;
}

function serve(root) {
  const server = http.createServer(async (req, res) => {
    const pathname = decodeURIComponent(new URL(req.url, "http://x").pathname);
    const file = path.join(root, pathname.endsWith("/") ? `${pathname}index.html` : pathname);
    if (!file.startsWith(root)) { res.writeHead(403).end(); return; }
    try {
      const body = await fs.readFile(file);
      res.writeHead(200, { "content-type": MIME[path.extname(file)] ?? "application/octet-stream" });
      res.end(body);
    } catch { res.writeHead(404).end(); }
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}


// The release site opens on the intro movies (crates/rebellion-app/src/main.rs,
// FUN_004138a0); Escape skips each one (cutscenes.mjs) until the main menu
// is active.
async function skipToMainMenu(page, timeout = 90_000) {
  const deadline = Date.now() + timeout;
  for (;;) {
    const active = await page.locator("#main-menu-semantics[data-active=true]").count();
    if (active > 0) return;
    if (Date.now() > deadline) throw new Error("the main menu never became active");
    if (await page.evaluate(() => Boolean(document.querySelector("video")))) {
      await page.keyboard.press("Escape");
    }
    await page.waitForTimeout(500);
  }
}

async function waitFor(page, log, description, predicate, timeout) {
  const deadline = Date.now() + timeout;
  while (!predicate()) {
    if (Date.now() > deadline) {
      throw new Error(`${description}: last lines ${JSON.stringify(log.slice(-8))}`);
    }
    await page.waitForTimeout(100);
  }
}

async function frames(page, count = 3) {
  await page.evaluate((n) => new Promise((resolve) => {
    let left = n;
    const step = () => (--left <= 0 ? resolve() : requestAnimationFrame(step));
    requestAnimationFrame(step);
  }), count);
}

// egui resolves a click across frames: hover, press, then release.
async function click(page, [x, y], button = "left") {
  await page.mouse.move(x, y);
  await frames(page);
  await page.mouse.down({ button });
  await frames(page);
  await page.mouse.up({ button });
  await frames(page);
}

async function run(browser, base, faction) {
  const directory = path.join(OUT, faction.name);
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const log = [];
  const errors = [];
  const network = [];
  const shots = [];
  page.on("console", (message) => {
    log.push(message.text());
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  page.on("response", (response) => network.push({ url: new URL(response.url()).pathname,
    status: response.status() }));
  const shot = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    shots.push({ label, sha256: createHash("sha256").update(bytes).digest("hex") });
  };
  try {
    await page.goto(base);
    // The release site plays the intro first (MDATA.000, 001): Escape
    // skips each movie until the main menu shows.
    await skipToMainMenu(page);
    await page.waitForTimeout(1_000);
    await page.evaluate((index) => wasm_exports.open_rebellion_menu_activate(index), faction.menuIndex);
    await waitFor(page, log, "the new game starts",
      () => log.some((line) => line.includes("[campaign] faction=")), 60_000);
    await waitFor(page, log, "the briefing starts",
      () => log.some((line) => line.includes("[advisor] clip voice")), 20_000);
    await click(page, [320, 240]);
    await waitFor(page, log, "the short form ends",
      () => log.some((line) => /clip finished kind=2 code=0xd /.test(line)), 60_000);
    await page.waitForTimeout(300);
    // The briefing opens the Message Index on Advice, and the index is
    // modal: in A0 (a0-vm/tooltip/171, 172) neither the cockpit nor F3/F7
    // act under it. Escape closes it.
    if (log.some((line) => line.includes("status=opened_by_briefing"))) {
      await page.keyboard.press("Escape");
      await page.waitForTimeout(400);
    }
    await page.mouse.move(2, 2);
    if (args.window === "finders") {
      const finderText = {};
      // F3 the Fleet Finder, F4 the Troop Finder, F5 the Personnel Finder
      // (FUN_00422ce0; A0 2026-10-29 opens them so), each closed by Escape.
      for (const [key, label] of [["F3", "fleet-finder"], ["F4", "troop-finder"], ["F5", "personnel-finder"]]) {
        await page.keyboard.down(key);
        await frames(page);
        await page.keyboard.up(key);
        await page.waitForTimeout(800);
        await shot(label);
        if (args.a0 && faction.name === "empire") {
          finderText[label] = await textAgainstA0(path.join(directory, `${label}.png`),
            FINDER_A0[label], FINDER_TEXT);
        }
        if (label === "troop-finder" && faction.name === "alliance") {
          // FUN_00609650 on each typed character: the first row whose name
          // the typed text prefixes is chosen, white (A0 finder-search/02,
          // 03: "P" then "Pz" choose Pzob).
          // The port's fixture lists Bpfassh, Ghorman, Ithor and Yavin, rows
          // 25 apart from y 191; "I" chooses Ithor, the third.
          await page.keyboard.type("I", { delay: 80 });
          await page.waitForTimeout(400);
          await shot("troop-finder-typed");
          const typed = PNG.sync.read(await fs.readFile(path.join(directory, "troop-finder-typed.png")));
          const brightest = (row) => {
            let top = 0;
            for (let y = 196 + 25 * row; y < 196 + 25 * row + 12; y += 1) {
              for (let x = 102; x < 250; x += 1) {
                const i = (y * typed.width + x) * 4;
                top = Math.max(top, Math.min(typed.data[i], typed.data[i + 1], typed.data[i + 2]));
              }
            }
            return top;
          };
          finderText.typed = { rows: [0, 1, 2, 3].map(brightest) };
          assert.ok(finderText.typed.rows[2] > 230 && finderText.typed.rows[0] < 200,
            `"I" chooses Ithor: ${JSON.stringify(finderText.typed)}`);
          // Closed and opened again, the Finder forgets the choice and
          // chooses its first row (A0 f4-reopened-day171, FUN_00609650 on
          // the empty name).
          await page.keyboard.press("Escape");
          await page.waitForTimeout(400);
          await page.keyboard.down("F4");
          await frames(page);
          await page.keyboard.up("F4");
          await page.waitForTimeout(800);
          await shot("troop-finder-reopened");
          const reopened = PNG.sync.read(await fs.readFile(path.join(directory, "troop-finder-reopened.png")));
          const rowTop = (row) => {
            let top = 0;
            for (let y = 196 + 25 * row; y < 196 + 25 * row + 12; y += 1) {
              for (let x = 102; x < 250; x += 1) {
                const i = (y * reopened.width + x) * 4;
                top = Math.max(top, Math.min(reopened.data[i], reopened.data[i + 1], reopened.data[i + 2]));
              }
            }
            return top;
          };
          finderText.reopened = { rows: [0, 1, 2, 3].map(rowTop) };
          assert.ok(finderText.reopened.rows[0] > 230 && finderText.reopened.rows[2] < 200,
            `the reopened Finder chooses its first row: ${JSON.stringify(finderText.reopened)}`);
          // Down and Up step the chosen row (A0 finder-keys/t0..t3: Drall,
          // Down to the second row, Up back).
          const stepTo = async (key) => {
            await page.keyboard.press(key);
            await page.waitForTimeout(400);
            const png = PNG.sync.read(await page.screenshot());
            const top = (row) => {
              let best = 0;
              for (let y = 196 + 25 * row; y < 196 + 25 * row + 12; y += 1) {
                for (let x = 102; x < 250; x += 1) {
                  const i = (y * png.width + x) * 4;
                  best = Math.max(best, Math.min(png.data[i], png.data[i + 1], png.data[i + 2]));
                }
              }
              return best;
            };
            return [0, 1, 2].map(top);
          };
          finderText.down = await stepTo("ArrowDown");
          finderText.up = await stepTo("ArrowUp");
          assert.ok(finderText.down[1] > 230 && finderText.down[0] < 200 && finderText.up[0] > 230 && finderText.up[1] < 200,
            `Down and Up step the Troop Finder's row: ${JSON.stringify({ down: finderText.down, up: finderText.up })}`);
          // The other side's tab (0x6f at the window's (88, 72)), with the
          // name box empty, chooses its first row (FUN_0046ce40's
          // FUN_0060d7e0(side, 1); A0 troop-finder-first-row/alliance-tab:
          // Yavin, white).
          await page.mouse.move(62 + 88 + 24, 50 + 72 + 20);
          await frames(page);
          await page.mouse.down();
          await frames(page);
          await page.mouse.up();
          await frames(page);
          await page.mouse.move(2, 2);
          await page.waitForTimeout(400);
          await shot("troop-finder-other-tab");
          const other = PNG.sync.read(await fs.readFile(path.join(directory, "troop-finder-other-tab.png")));
          const otherTop = (row) => {
            let top = 0;
            for (let y = 196 + 25 * row; y < 196 + 25 * row + 12; y += 1) {
              for (let x = 102; x < 250; x += 1) {
                const i = (y * other.width + x) * 4;
                top = Math.max(top, Math.min(other.data[i], other.data[i + 1], other.data[i + 2]));
              }
            }
            return top;
          };
          // The heading's band (the window's (40..140, 128..140)) changes
          // with the tab: "Alliance Troops" to "Imperial Troops".
          const heading = (png) => {
            let sum = 0;
            for (let y = 178; y < 190; y += 1) {
              for (let x = 102; x < 202; x += 1) {
                const i = (y * png.width + x) * 4;
                sum += png.data[i] + png.data[i + 1] + png.data[i + 2];
              }
            }
            return sum;
          };
          finderText.otherTab = { rows: [0, 1, 2, 3].map(otherTop),
            heading: [heading(reopened), heading(other)] };
          assert.notEqual(finderText.otherTab.heading[0], finderText.otherTab.heading[1],
            "the tab did not change");
          // The tab lists only the regiments the player's side sees (A0
          // troop-other-side/test-day220-imperial-tab-empty), and this
          // galaxy's fog moves while it runs: either no row (the list's
          // background, under 100) or the first row chosen.
          const otherRows = finderText.otherTab.rows;
          assert.ok((otherRows[0] > 230 && otherRows.slice(1).every((top) => top < 200))
            || otherRows.every((top) => top < 100),
            `the other side's tab chooses its first row or lists none: ${JSON.stringify(finderText.otherTab)}`);
        }
        if (label === "personnel-finder" && faction.name === "alliance") {
          // The Alliance's Personnel Finder against A0
          // specforces/00-personnel-finder-alliance-day161 (both at
          // (62, 50)): every pixel of the window but the list, its bar, the
          // name box and the three texts (GDI hinting, the accepted text
          // deviation), at most 4 differing.
          try {
            const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
              ".artifacts/a0-vm/specforces/00-personnel-finder-alliance-day161.png")));
            const port = PNG.sync.read(await fs.readFile(path.join(directory, "personnel-finder.png")));
            const skip = [[200, 95, 452, 117], [98, 185, 460, 360], [200, 64, 340, 82], [95, 98, 165, 113], [100, 166, 240, 184]];
            let differing = 0;
            for (let y = 50; y < 380; y += 1) {
              for (let x = 62; x < 505; x += 1) {
                if (skip.some(([l, t, r, b]) => x >= l && x <= r && y >= t && y <= b)) continue;
                const i = (y * port.width + x) * 4;
                if (port.data[i] !== a0.data[i] || port.data[i + 1] !== a0.data[i + 1] || port.data[i + 2] !== a0.data[i + 2]) differing += 1;
              }
            }
            finderText.allianceChrome = { differing };
            assert.ok(differing <= 4, `the Alliance's Personnel Finder differs from A0 in ${differing} pixels`);
          } catch (error) {
            if (error.code !== "ENOENT") throw error;
          }
          finderText.specforces = await specForcesView(page, shot, directory);
        }
        await page.keyboard.press("Escape");
        await page.waitForTimeout(400);
      }
      // The Fleet window: F3, the first row, then the rail's Display (0xc9,
      // fleet-finder.md: Alliance (423, 93), Empire (426, 89)).
      const [fx, fy] = ORIGINS[faction.name];
      const display = faction.name === "empire" ? [426 + 22, 89 + 20] : [423 + 16, 93 + 15];
      await page.keyboard.down("F3");
      await frames(page);
      await page.keyboard.up("F3");
      await page.waitForTimeout(500);
      await click(page, [fx + 100, fy + 138 + 10]);
      await click(page, [fx + display[0], fy + display[1]]);
      await page.mouse.move(2, 2);
      await page.waitForTimeout(800);
      await shot("fleet-window");
      return { faction: faction.name, status: "pass", shots, finderText };
    }
    await page.keyboard.down("F7");
    await frames(page);
    await page.keyboard.up("F7");
    await waitFor(page, log, "F7 opens the original Encyclopedia",
      () => log.some((line) => line.includes("destination=encyclopedia status=opened_original")), 5_000);
    await page.waitForTimeout(800);
    await shot("encyclopedia-index");
    const indexText = args.a0 && faction.name === "empire"
      ? await textAgainstA0(path.join(directory, "encyclopedia-index.png"), "index-empire-day143.png", INDEX_TEXT) : null;
    // The bar: three clicks on the down arrow scroll the list.
    const [ox, oy] = ORIGINS[faction.name];
    const list = { x: ox + 36, y: oy + 137, width: 338, height: 160 };
    const before = await page.screenshot({ clip: list });
    for (let index = 0; index < 3; index += 1) await click(page, [ox + 380, oy + 292]);
    await page.mouse.move(2, 2);
    await page.waitForTimeout(300);
    const after = await page.screenshot({ clip: list });
    assert.ok(!before.equals(after), "the down arrow scrolls the list");
    await shot("encyclopedia-index-scrolled");
    // The topic: the list back at its top (three clicks on the up arrow),
    // Abduction (the second row) selected, then the rail's topic 0x67.
    const rail = faction.name === "empire" ? [426 + 22, 89 + 20] : [423 + 16, 93 + 15];
    for (let index = 0; index < 3; index += 1) await click(page, [ox + 380, oy + 137 + 4]);
    await click(page, [ox + 100, oy + 137 + 30]);
    await click(page, [ox + rail[0], oy + rail[1]]);
    await waitFor(page, log, "the rail's topic opens Abduction",
      () => log.some((line) => /\[encyclopedia\] command=0x67 topic_mode=true/.test(line)), 5_000);
    await page.mouse.move(2, 2);
    await page.waitForTimeout(800);
    await shot("encyclopedia-topic");
    const topicText = args.a0 && faction.name === "empire"
      ? await textAgainstA0(path.join(directory, "encyclopedia-topic.png"), "topic-abduction-empire-day166.png", TOPIC_TEXT) : null;
    const allDatabases = faction.name === "alliance" ? await allDatabasesFleet(page, log, shot, directory) : null;
    assert.deepEqual(errors, [], errors.join("; "));
    assert.ok(network.every(({ status }) => status < 400),
      JSON.stringify(network.filter(({ status }) => status >= 400)));
    return { faction: faction.name, status: "pass", shots, indexText, topicText, allDatabases };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: faction.name, status: "fail", error: String(error.stack || error), errors };
  } finally {
    await fs.writeFile(path.join(directory, "console.log"), log.join("\n"));
    await context.close();
  }
}

await fs.mkdir(OUT, { recursive: true });
const server = await serve(WEB);
const base = `http://127.0.0.1:${server.address().port}/`;
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium",
  headless: true,
  args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
const results = [];
try {
  for (const faction of FACTIONS) results.push(await run(browser, base, faction));
} finally {
  await browser.close();
  server.close();
}
const wasm = await fs.readFile(path.join(WEB, "open-rebellion.wasm"));
const summary = {
  status: results.every(({ status }) => status === "pass") ? "pass" : "fail",
  wasm_sha256: createHash("sha256").update(wasm).digest("hex"),
  results,
};
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: summary.status,
  results: results.map(({ faction, status, error }) => ({ faction, status, error })) }, null, 2));
if (summary.status !== "pass") process.exitCode = 1;
