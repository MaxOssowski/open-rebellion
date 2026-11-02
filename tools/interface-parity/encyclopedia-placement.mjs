// The Encyclopedia's placement in the browser build
// (ghidra/notes/encyclopedia-window.md, "Production routing").
//
// Serves web/, starts a new game as each faction in headless Chromium at
// 640×480, skips the opening briefing and presses F7 (command 0x131), which
// opens the original index. The screenshot is kept for comparison with the
// A0 capture (.artifacts/a0-vm/encyclopedia/), whose window sits at
// (125, 52) for the Empire; captures 281117 and 281123 put the Alliance's
// at (62, 50).
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
        if (label === "personnel-finder" && faction.name === "alliance") {
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
    assert.deepEqual(errors, [], errors.join("; "));
    assert.ok(network.every(({ status }) => status < 400),
      JSON.stringify(network.filter(({ status }) => status >= 400)));
    return { faction: faction.name, status: "pass", shots, indexText, topicText };
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
