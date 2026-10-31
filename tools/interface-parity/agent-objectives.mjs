// The Agent menu and its Objectives in the browser build
// (ghidra/notes/agent-menu.md, ghidra/notes/status-window.md).
//
// Serves web/, starts a new game as each faction in headless Chromium at
// 640×480, skips the opening briefing, then:
//
// - "menu": a right click on the agent droid opens the Agent menu; a click
//   on its Objectives row opens the Status window on the empty id;
// - "key": Alt+H opens the same window;
// - the menu's Galaxy Overview row and Alt+O open the Status window on the
//   galaxy;
// - Alt+G's reaction plays, then at Fast speed fifty quiet sub-ticks
//   (FUN_004fcee0; 4 a day at Fast, about 13 days) bring a
//   quip, an op-0x13 clip (FUN_00412c40).
//
// Screenshots of the menu and the window are kept for comparison with the
// A0 captures (.artifacts/a0-vm/agent-menu/).
//
// Build first: scripts/build-wasm.sh
// Run:         node agent-objectives.mjs [--out=DIR] [--faction=alliance|empire]
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
  `agent-objectives-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// The main menu's entries (galaxy-map-visual-check.mjs) and a point on each
// side's agent droid (advisor.rs agent_aperture: Alliance (541,337) 67×116,
// Empire (0,347) 107×133), and a point on the menu's Objectives row: the
// menu ends at the release point, on the droid's right for the Empire and
// on its left for the Alliance. The port's row is at y 307; the A0 capture
// of the Empire's menu, opened at (46, 401), has it at y 311.
const FACTIONS = [
  { name: "alliance", menuIndex: 9, droid: [570, 400], objectives: [480, 307], overview: [480, 287],
    garrisonsVoice: "directory=alsprite-dll wave=1137", quipVoice: "directory=alsprite-dll wave=1291",
    dayReadout: [144, 27], fastRow: [180, 117] },
  { name: "empire", menuIndex: 8, droid: [46, 401], objectives: [100, 307], overview: [100, 287],
    garrisonsVoice: "directory=emsprite-dll wave=1135", quipVoice: "directory=emsprite-dll wave=22005",
    dayReadout: [539, 25], fastRow: [575, 115] },
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
  // The wall-clock times of the [ai] day lines at Fast.
  const fastDays = [];
  const errors = [];
  const network = [];
  const shots = [];
  page.on("console", (message) => {
    log.push(message.text());
    if (/\[ai\] day=\d+ speed=Fast/.test(message.text())) fastDays.push(Date.now());
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  page.on("response", (response) => network.push({ url: new URL(response.url()).pathname,
    status: response.status() }));
  const shot = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    shots.push({ label, sha256: createHash("sha256").update(bytes).digest("hex") });
  };
  const opened = () => log.filter((line) => line.includes("[agent] command=objectives status=opened"));
  const closed = () => log.filter((line) => line.includes("destination=status_window status=closed"));
  try {
    await page.goto(base);
    // The release site plays the intro first (MDATA.000, 001): Escape
    // skips each movie until the main menu shows.
    await skipToMainMenu(page);
    await page.waitForTimeout(1_000);
    await page.evaluate((index) => wasm_exports.open_rebellion_menu_activate(index), faction.menuIndex);
    await waitFor(page, log, "the new game starts",
      () => log.some((line) => line.includes("[campaign] faction=")), 60_000);
    // Skip the briefing (a click), then wait for its last action 0xd.
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

    // The menu: right click on the droid.
    await click(page, faction.droid, "right");
    await page.waitForTimeout(400);
    await shot("agent-menu");
    await click(page, faction.objectives);
    await waitFor(page, log, "the menu's Objectives opens the window", () => opened().length === 1, 5_000);
    assert.match(opened()[0], /source=menu/);
    await page.mouse.move(2, 2);
    await page.waitForTimeout(300);
    await shot("objectives-from-menu");
    await page.keyboard.press("Escape");
    await waitFor(page, log, "Escape closes it", () => closed().length === 1, 5_000);

    // Alt+H.
    await page.keyboard.down("Alt");
    await frames(page);
    await page.keyboard.down("KeyH");
    await frames(page);
    await page.keyboard.up("KeyH");
    await page.keyboard.up("Alt");
    await waitFor(page, log, "Alt+H opens the window", () => opened().length === 2, 5_000);
    assert.match(opened()[1], /source=accelerator/);
    await page.waitForTimeout(300);
    await shot("objectives-from-alt-h");
    await page.keyboard.press("Escape");
    await waitFor(page, log, "Escape closes it", () => closed().length === 2, 5_000);

    // The Galaxy Overview: the menu's fourth row, then Alt+O.
    const overview = () => log.filter((line) => line.includes("[agent] command=galaxy_overview status=opened"));
    await click(page, faction.droid, "right");
    await page.waitForTimeout(400);
    await click(page, faction.overview);
    await waitFor(page, log, "the menu's Galaxy Overview opens the window", () => overview().length === 1, 5_000);
    assert.match(overview()[0], /source=menu/);
    await page.mouse.move(2, 2);
    await page.waitForTimeout(300);
    await shot("galaxy-overview-from-menu");
    await page.keyboard.press("Escape");
    await waitFor(page, log, "Escape closes it", () => closed().length === 3, 5_000);
    await page.keyboard.down("Alt");
    await frames(page);
    await page.keyboard.down("KeyO");
    await frames(page);
    await page.keyboard.up("KeyO");
    await page.keyboard.up("Alt");
    await waitFor(page, log, "Alt+O opens the window", () => overview().length === 2, 5_000);
    assert.match(overview()[1], /source=accelerator/);
    await page.keyboard.press("Escape");
    await page.waitForTimeout(300);

    // Translate Counterpart: on in a new game (0x1000, FUN_004c27f0 /
    // FUN_004c0710), Alt+V (0xbc8 -> 0x11b) flips it off, then on.
    const translate = () => log.filter((line) => line.includes("[agent] command=translate_counterpart"));
    for (const expected of ["off", "on"]) {
      await page.keyboard.down("Alt");
      await frames(page);
      await page.keyboard.down("KeyV");
      await frames(page);
      await page.keyboard.up("KeyV");
      await page.keyboard.up("Alt");
      await waitFor(page, log, `Alt+V turns it ${expected}`,
        () => translate().some((line) => line.includes(`status=${expected}`)), 5_000);
    }
    await click(page, faction.droid, "right");
    await page.waitForTimeout(400);
    await shot("agent-menu-translate");
    await page.keyboard.press("Escape");

    // A reaction: Alt+G turns Manage Garrisons on (FUN_00439d60, 0x10);
    // once nothing is due, FUN_004c2dd0 / FUN_004c0d00 schedule message
    // 0x24 / 0x1d, which plays its sprite-DLL voice at the next idle end.
    await page.keyboard.down("Alt");
    await frames(page);
    await page.keyboard.down("KeyG");
    await frames(page);
    await page.keyboard.up("KeyG");
    await page.keyboard.up("Alt");
    await waitFor(page, log, "the flags reach the advisor",
      () => log.some((line) => line.includes("[advisor] reaction flags=0x10")), 5_000);
    await waitFor(page, log, "the droid answers the garrisons",
      () => log.some((line) => line.includes(`[advisor] clip voice ${faction.garrisonsVoice}`)), 30_000);

    // A quip: once fifty sub-ticks pass without a message, each idle end sets
    // 0x80000000 (FUN_00439ef0) and the advisor rolls FUN_005f5700(12)
    // below 6. The port's generator rolls 6, 7, then 4 from its seed, so
    // the third idle end plays message 0x54 / 0x52, record 905 (op 0x13).
    // The game opens paused, so Fast (1 s a day) comes from the Game
    // Speed menu on the day readout (FUN_0042d190, game_speed.rs).
    await click(page, faction.dayReadout, "right");
    await page.waitForTimeout(300);
    await shot("speed-menu");
    await click(page, faction.fastRow);
    const quip = () => log.some((line) => line.includes(`[advisor] clip voice ${faction.quipVoice}`));
    const battles = () => log.filter((line) => line.includes("[audio] context=battle_window")).length;
    const deadline = Date.now() + 240_000;
    let handled = 0;
    while (!quip()) {
      if (Date.now() > deadline) throw new Error(`fifty quiet sub-ticks bring a quip: ${JSON.stringify(log.slice(-8))}`);
      if (battles() > handled) {
        handled = battles();
        // The Battle Alert's Simulate, then the results' close box
        // (tactical_view.rs: Simulate in the bottom bar, the close box
        // at the rail's top).
        await page.waitForTimeout(1_000);
        await click(page, [270, 357]);
        await page.waitForTimeout(3_000);
        await shot(`battle-${handled}-result`);
        await click(page, [500, 82]);
        await page.waitForTimeout(500);
      }
      await page.waitForTimeout(100);
    }
    await page.waitForTimeout(300);
    await shot("quip");
    // Fast is 1 s a day (4 quarter-second sub-ticks, FUN_00415e20; A0 1.26
    // s, ghidra/notes/game-speed-timing.md). A battle's alert stops the
    // clock, so only the median spacing is asserted.
    const spacing = fastDays.slice(1).map((at, index) => (at - fastDays[index]) / 1000).sort((a, b) => a - b);
    const median = spacing[Math.floor(spacing.length / 2)];
    assert.ok(spacing.length >= 4 && median > 0.85 && median < 1.15,
      `Fast days ${JSON.stringify(spacing)} s apart, median ${median}, not 1 s`);
    assert.deepEqual(errors, [], errors.join("; "));
    assert.ok(network.every(({ status }) => status < 400),
      JSON.stringify(network.filter(({ status }) => status >= 400)));
    return { faction: faction.name, status: "pass", agent: log.filter((line) => line.startsWith("[agent]")), shots,
      fast_day_median_seconds: median };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: faction.name, status: "fail", error: String(error.stack || error),
      agent: log.filter((line) => line.startsWith("[agent]")), errors };
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
