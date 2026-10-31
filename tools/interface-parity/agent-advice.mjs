// The agent's advice in the browser build (ghidra/notes/agent-advice-list.md).
//
// Serves web/, starts a new game as each faction in headless Chromium at
// 640×480 (the main menu's default difficulty is Easy, which turns Agent
// Advice on: FUN_00439320), and skips the opening briefing. The short
// form's closing action 0xd (FUN_004c30c0 / FUN_004c0fc0) builds the
// agent's list (FUN_00439f20), which posts the nine kind-7 advice, and
// opens the Message Index on Advice (FUN_0041d770(1, 0x82)). The gate
// checks the nine posts and the index, opens the newest advice, and keeps
// screenshots for comparison with the A0 captures
// (.artifacts/a0-vm/agent-advice/).
//
// Build first: scripts/build-wasm.sh
// Run:         node agent-advice.mjs [--out=DIR] [--faction=alliance|empire]
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
  `agent-advice-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// The main menu's entries (galaxy-map-visual-check.mjs) and the index's
// first row (the list child at (22, 108) of the galaxy-centred 470 by 331
// window, message_index.rs).
// The window sits centred in the galaxy view (Alliance (62, 49), Empire
// (125, 52)); Select All (0x90) at (0x11a, 0x57) 56 by 20 and the posting
// toggle (0x66) on the rail at (0x1a7, 0x93) 32 by 31 / (0x1aa, 0x94) 44 by
// 41 (FUN_004665f0).
const FACTIONS = [
  { name: "alliance", menuIndex: 9, firstRow: [args.alliance_row_x ?? 260, args.alliance_row_y ?? 160],
    selectAll: [62 + 0x11a + 28, 49 + 0x57 + 10], post: [62 + 0x1a7 + 16, 49 + 0x93 + 15],
    secondDroid: [316 + 23, 411 + 34] },
  { name: "empire", menuIndex: 8, firstRow: [args.empire_row_x ?? 330, args.empire_row_y ?? 160],
    selectAll: [125 + 0x11a + 28, 52 + 0x57 + 10], post: [125 + 0x1aa + 22, 52 + 0x94 + 20],
    secondDroid: [302 + 50, 401 + 39] },
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
  const posted = () => log.flatMap((line) => {
    const match = line.match(/\[agent\] advice posted number=(\d+) kind=(\d+) reason=(\w+) titled=(\w+)/);
    return match ? [{ number: Number(match[1]), kind: Number(match[2]), reason: match[3],
      titled: match[4] === "true" }] : [];
  });
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
    assert.equal(posted().length, 0, "no advice before the briefing ends");
    await click(page, [320, 240]);
    await waitFor(page, log, "the short form ends",
      () => log.some((line) => /clip finished kind=2 code=0xd /.test(line)), 60_000);
    await waitFor(page, log, "the index opens on Advice",
      () => log.some((line) => line.includes("category=0x82 status=opened_by_briefing")), 5_000);
    // FUN_00439f20: the kind-7 advice, posted from the last node back (9
    // down to 1), all titled, so Victory Conditions heads the list (A0
    // new-alliance/t25).
    assert.deepEqual(posted().map(({ number }) => number), [9, 8, 7, 6, 5, 4, 3, 2, 1]);
    assert.ok(posted().every(({ kind, reason, titled }) => kind === 7 && reason === "briefing" && titled),
      JSON.stringify(posted()));
    await page.mouse.move(2, 2);
    await page.waitForTimeout(500);
    await shot("advice-index");
    // Select All (0x90) chooses every row; the posting toggle (0x66,
    // FUN_00467f10) silences Advice (mask 0x200), then turns it back.
    await click(page, faction.selectAll);
    await page.mouse.move(2, 2);
    await page.waitForTimeout(300);
    await shot("select-all");
    const posting = () => log.filter((line) => line.includes("[interface] message_index posting"));
    await click(page, faction.post);
    await waitFor(page, log, "the toggle silences Advice", () => posting().length === 1, 5_000);
    assert.match(posting()[0], /mask=Some\(512\) silent=true/);
    await page.mouse.move(2, 2);
    await page.waitForTimeout(300);
    await shot("posting-silent");
    await click(page, faction.post);
    await waitFor(page, log, "the toggle turns the alerts back on", () => posting().length === 2, 5_000);
    assert.match(posting()[1], /mask=Some\(512\) silent=false/);
    // The newest advice (9, "Advice Available") heads the list.
    await click(page, faction.firstRow);
    await click(page, faction.firstRow);
    await page.mouse.move(2, 2);
    await page.waitForTimeout(500);
    await shot("advice-message");
    assert.equal(posted().length, 9, "nothing more is posted at once");
    // The Message Index is modal (A0 a0-vm/modal2/18..21: the droids do
    // nothing under it), so it closes first.
    await page.keyboard.press("Escape");
    await page.waitForTimeout(300);
    // The second droid's panel (FUN_0042d050, 0xbeef) opens the Message
    // Alerts menu (FUN_00487a60): "Messages" and the "Message Alerts"
    // submenu.
    await click(page, faction.secondDroid, "right");
    await waitFor(page, log, "the second droid opens the Message Alerts menu",
      () => log.some((line) => line.includes("[interface] message_alerts status=opened")), 5_000);
    await page.mouse.move(2, 2);
    await page.waitForTimeout(300);
    await shot("alerts-menu");
    await page.keyboard.press("Escape");
    assert.deepEqual(errors, [], errors.join("; "));
    assert.ok(network.every(({ status }) => status < 400),
      JSON.stringify(network.filter(({ status }) => status >= 400)));
    return { faction: faction.name, status: "pass", posted: posted(), shots,
      agent: log.filter((line) => line.startsWith("[agent]") || line.includes("message_index")) };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: faction.name, status: "fail", error: String(error.stack || error),
      agent: log.filter((line) => line.startsWith("[agent]") || line.includes("message_index")), errors };
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
