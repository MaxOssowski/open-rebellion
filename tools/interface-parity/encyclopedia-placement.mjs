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
const FACTIONS = [
  { name: "alliance", menuIndex: 9, droid: [570, 400], objectives: [480, 307], overview: [480, 287] },
  { name: "empire", menuIndex: 8, droid: [46, 401], objectives: [100, 307], overview: [100, 287] },
].filter(({ name }) => !args.faction || args.faction === name);
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".wav": "audio/wav", ".bmp": "image/bmp" };

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
    await page.waitForSelector("#main-menu-semantics[data-active=true]",
      { state: "attached", timeout: 90_000 });
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
    await page.mouse.move(2, 2);
    if (args.window === "finders") {
      // F3 the Fleet Finder, F4 the Troop Finder, F5 the Personnel Finder
      // (FUN_00422ce0; A0 2026-10-29 opens them so), each closed by Escape.
      for (const [key, label] of [["F3", "fleet-finder"], ["F4", "troop-finder"], ["F5", "personnel-finder"]]) {
        await page.keyboard.down(key);
        await frames(page);
        await page.keyboard.up(key);
        await page.waitForTimeout(800);
        await shot(label);
        await page.keyboard.press("Escape");
        await page.waitForTimeout(400);
      }
      return { faction: faction.name, status: "pass", shots };
    }
    await page.keyboard.down("F7");
    await frames(page);
    await page.keyboard.up("F7");
    await waitFor(page, log, "F7 opens the original Encyclopedia",
      () => log.some((line) => line.includes("destination=encyclopedia status=opened_original")), 5_000);
    await page.waitForTimeout(800);
    await shot("encyclopedia-index");
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
    assert.deepEqual(errors, [], errors.join("; "));
    assert.ok(network.every(({ status }) => status < 400),
      JSON.stringify(network.filter(({ status }) => status >= 400)));
    return { faction: faction.name, status: "pass", shots };
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
