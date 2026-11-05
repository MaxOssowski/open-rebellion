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
import { PNG } from "pngjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

// The posting toggle's 32 by 31 cell at the screen's (485, 196) (the
// index at (62, 49), 0x66 at its (0x1a7, 0x93)) against an A0 capture:
// the differing pixels, or null without the capture.
async function toggleAgainstA0(page, file) {
  let a0;
  try {
    a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/message-silent", file)));
  } catch {
    return null;
  }
  const screen = PNG.sync.read(await page.screenshot());
  let differ = 0;
  for (let y = 196; y < 227; y += 1) {
    for (let x = 485; x < 517; x += 1) {
      const i = (y * 640 + x) * 4;
      if ([0, 1, 2].some((c) => screen.data[i + c] !== a0.data[i + c])) differ += 1;
    }
  }
  return differ;
}

// The Alliance Advice rail control (0x13d, 27 by 22 at (3, 284)) against
// an A0 capture of a new Easy game (message-rail/): the differing pixels,
// or null without the capture.
async function adviceRailAgainstA0(page, file) {
  let a0;
  try {
    a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/message-rail", file)));
  } catch {
    return null;
  }
  const screen = PNG.sync.read(await page.screenshot());
  let differ = 0;
  for (let y = 284; y < 306; y += 1) {
    for (let x = 3; x < 30; x += 1) {
      const i = (y * 640 + x) * 4;
      if ([0, 1, 2].some((c) => screen.data[i + c] !== a0.data[i + c])) differ += 1;
    }
  }
  return differ;
}
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
  // Intermediate (control 1) before the Alliance: FUN_00439320 leaves
  // Agent Advice off (A0 agent-menu/alliance-intermediate-day2 and
  // alliance-expert-day1 bare, alliance-easy-day1 checked).
  { name: "alliance-intermediate", menuIndex: 9, difficulty: 1, droid: [570, 400], adviceRow: [480, 387] },
].filter(({ name }) => !args.faction || args.faction === name);
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".wav": "audio/wav", ".bmp": "image/bmp" };

// The unread advice rows (FUN_00468ab0: font 13, set by
// theme::paint_gdi_text) against A0 tooltip/170-index (--a0=.artifacts/
// a0-vm/tooltip; the Empire's index, its rows other topics): the first
// row's ink starts 24 right of the heading's and its capitals 40 below the
// heading's top, within a pixel. Both windows' contents differ, so only
// these offsets are compared.
async function rowsAgainstA0(portFile) {
  const read = async (file) => PNG.sync.read(await fs.readFile(file));
  const ink = (png, [x0, y0, x1, y1], pred) => {
    let box = null;
    for (let y = y0; y < y1; y += 1) {
      for (let x = x0; x < x1; x += 1) {
        const i = (y * png.width + x) * 4;
        if (!pred(png.data[i], png.data[i + 1], png.data[i + 2])) continue;
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
      }
    }
    return box;
  };
  const white = (r, g, b) => Math.min(r, g, b) > 200;
  const grey = (r, g, b) => r > 100 && Math.abs(r - g) < 25 && Math.abs(g - b) < 40 && b > 100;
  const a0 = await read(path.join(args.a0, "170-index.png"));
  const port = await read(portFile);
  const offsets = (png, heading, row) => {
    const h = ink(png, heading, white);
    const r = ink(png, row, grey);
    return [r[0] - h[0], r[1] - h[1]];
  };
  const check = { a0: offsets(a0, [150, 142, 300, 158], [175, 180, 500, 200]),
    port: offsets(port, [90, 140, 300, 156], [112, 177, 430, 197]) };
  assert.ok(check.a0.every((value, k) => Math.abs(value - check.port[k]) <= 1), `advice rows: ${JSON.stringify(check)}`);
  return check;
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
    if (faction.difficulty !== undefined) {
      await page.evaluate((index) => wasm_exports.open_rebellion_menu_activate(index), faction.difficulty);
      await page.waitForTimeout(500);
    }
    await page.evaluate((index) => wasm_exports.open_rebellion_menu_activate(index), faction.menuIndex);
    await waitFor(page, log, "the new game starts",
      () => log.some((line) => line.includes("[campaign] faction=")), 60_000);
    await waitFor(page, log, "the briefing starts",
      () => log.some((line) => line.includes("[advisor] clip voice")), 20_000);
    assert.equal(posted().length, 0, "no advice before the briefing ends");
    await click(page, [320, 240]);
    await waitFor(page, log, "the short form ends",
      () => log.some((line) => /clip finished kind=2 code=0xd /.test(line)), 60_000);
    if (faction.difficulty !== undefined) {
      // No advice is posted and the index stays shut; the Agent menu's
      // last row, Agent Advice, shows no check mark beside it.
      await page.waitForTimeout(2_000);
      assert.equal(posted().length, 0, JSON.stringify(posted()));
      assert.ok(!log.some((line) => line.includes("status=opened_by_briefing")), "the index stays shut");
      await click(page, faction.droid, "right");
      await page.waitForTimeout(400);
      await shot("agent-menu");
      const png = PNG.sync.read(await page.screenshot());
      const white = (x, y) => {
        const i = (y * 640 + x) * 4;
        return png.data[i] === 255 && png.data[i + 1] === 255 && png.data[i + 2] === 255;
      };
      const [rowX, rowY] = faction.adviceRow;
      let border = rowX - 80;
      while (border < rowX && ![0, 2, 4, 6].every((dy) => white(border, rowY - 8 + dy)
        || white(border, rowY - 7 + dy))) border += 1;
      const cell = (y) => {
        let count = 0;
        for (let dy = -9; dy <= 9; dy += 1) {
          for (let x = border + 2; x < border + 22; x += 1) if (white(x, y + dy)) count += 1;
        }
        return count;
      };
      const checks = { advice: cell(rowY), translate: cell(rowY - 20) };
      assert.ok(checks.advice === 0 && checks.translate > 10, JSON.stringify(checks));
      assert.deepEqual(errors, [], errors.join("; "));
      return { faction: faction.name, status: "pass", posted: posted(), checks, shots };
    }
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
    // The rail lights Advice while its advice is unread (FUN_0048a2a0,
    // art 10067), the index open on it (A0 message-rail/easy-index-open).
    const rail = { open: faction.name === "alliance" ? await adviceRailAgainstA0(page, "easy-index-open.png") : null };
    const rowsText = args.a0 && faction.name === "alliance"
      ? await rowsAgainstA0(path.join(directory, "advice-index.png")) : null;
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
    // A0 message-silent/silent: Advice silenced, 0x2ac7 keyed on its first
    // pixel (FUN_005fd170), the rail through its border; alerting: 0x2a78.
    const toggle = { silent: faction.name === "alliance" ? await toggleAgainstA0(page, "silent.png") : null };
    await click(page, faction.post);
    await waitFor(page, log, "the toggle turns the alerts back on", () => posting().length === 2, 5_000);
    assert.match(posting()[1], /mask=Some\(512\) silent=false/);
    await page.mouse.move(2, 2);
    await page.waitForTimeout(300);
    toggle.alerting = faction.name === "alliance" ? await toggleAgainstA0(page, "alerting.png") : null;
    assert.ok((toggle.silent ?? 0) === 0 && (toggle.alerting ?? 0) === 0, JSON.stringify(toggle));
    // The newest advice (9, "Advice Available") heads the list.
    await click(page, faction.firstRow);
    await click(page, faction.firstRow);
    await page.mouse.move(2, 2);
    await page.waitForTimeout(500);
    await shot("advice-message");
    // A long advice shows the text control's bar (FUN_0041efe0:
    // FUN_0060a490(.., 0x299a, ..)) at its right edge: A0
    // agent-advice/02-advice-message (the Empire) has its up and down
    // arrows at (525, 286) and (525, 357), 13 by 9, every pixel; the
    // thumb follows the text, which differs.
    let textBar = null;
    if (faction.name === "empire") {
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/agent-advice/02-advice-message.png")));
        const screen = PNG.sync.read(await page.screenshot());
        const differ = (y0) => {
          let count = 0;
          for (let y = y0; y < y0 + 9; y += 1) {
            for (let x = 525; x < 538; x += 1) {
              const i = (y * 640 + x) * 4;
              if ([0, 1, 2].some((c) => screen.data[i + c] !== a0.data[i + c])) count += 1;
            }
          }
          return count;
        };
        textBar = { up: differ(286), down: differ(357) };
      } catch {
        textBar = null;
      }
      assert.ok(textBar === null || (textBar.up === 0 && textBar.down === 0), JSON.stringify(textBar));
      // The down arrow scrolls the text a line, 16 pixels (font 4's
      // tmHeight): the text's rows from 250 now show what stood at 266.
      const before = PNG.sync.read(await page.screenshot());
      await page.mouse.move(531, 361);
      await page.waitForTimeout(100);
      await page.mouse.down();
      await page.waitForTimeout(100);
      await page.mouse.up();
      await page.waitForTimeout(300);
      await page.mouse.move(2, 2);
      await page.waitForTimeout(300);
      await shot("advice-message-scrolled");
      const after = PNG.sync.read(await page.screenshot());
      let moved = 0;
      let changed = 0;
      for (let y = 290; y < 330; y += 1) {
        for (let x = 145; x < 520; x += 1) {
          const i = (y * 640 + x) * 4;
          const j = ((y + 16) * 640 + x) * 4;
          if ([0, 1, 2].some((c) => after.data[i + c] !== before.data[j + c])) moved += 1;
          if ([0, 1, 2].some((c) => after.data[i + c] !== before.data[i + c])) changed += 1;
        }
      }
      textBar = { ...(textBar ?? {}), scrolled_mismatch: moved, scrolled_changed: changed };
      assert.ok(moved === 0 && changed > 0, JSON.stringify(textBar));
    }
    assert.equal(posted().length, 9, "nothing more is posted at once");
    // The Message Index is modal (A0 a0-vm/modal2/18..21: the droids do
    // nothing under it), so it closes first.
    await page.keyboard.press("Escape");
    await page.waitForTimeout(300);
    // Closing the index marks Advice read (FUN_0048a610): the rail rests
    // (10057, A0 message-rail/easy-index-closed).
    await page.mouse.move(320, 200);
    await page.waitForTimeout(300);
    rail.closed = faction.name === "alliance" ? await adviceRailAgainstA0(page, "easy-index-closed.png") : null;
    assert.ok((rail.open ?? 0) === 0 && (rail.closed ?? 0) === 0, JSON.stringify(rail));
    // The second droid's panel (FUN_0042d050, 0xbeef) opens the Message
    // Alerts menu (FUN_00487a60): "Messages" and the "Message Alerts"
    // submenu.
    await click(page, faction.secondDroid, "right");
    await waitFor(page, log, "the second droid opens the Message Alerts menu",
      () => log.some((line) => line.includes("[interface] message_alerts status=opened")), 5_000);
    await page.mouse.move(2, 2);
    await page.waitForTimeout(300);
    await shot("alerts-menu");
    // Message Alerts' submenu lists the nine categories, each checked while
    // it alerts; a click flips its category (FUN_00486fb0). A0 (2026-11-06,
    // personnel-captured/r3, r5): all checked, then Fleets bare after a
    // click on it, the other eight still checked.
    let alerts = null;
    if (faction.name === "alliance") {
      const flips = () => log.filter((line) => line.includes("[interface] message_alerts mask="));
      const openSubmenu = async () => {
        await page.mouse.move(400, 436, { steps: 4 });
        await page.waitForTimeout(800);
      };
      const checkCells = async () => {
        const png = PNG.sync.read(await page.screenshot());
        return [278, 298, 318, 338, 358, 378, 398, 418, 438].map((y) => {
          let white = 0;
          for (let dy = -7; dy <= 7; dy += 1) {
            for (let x = 236; x < 254; x += 1) {
              const i = ((y + dy) * 640 + x) * 4;
              if (png.data[i] > 240 && png.data[i + 1] > 240 && png.data[i + 2] > 240) white += 1;
            }
          }
          return white > 10;
        });
      };
      await openSubmenu();
      await shot("alerts-submenu");
      const before = await checkCells();
      await page.mouse.move(370, 436, { steps: 2 });
      await page.mouse.move(342, 436, { steps: 2 });
      await page.mouse.move(300, 298, { steps: 4 });
      await page.waitForTimeout(300);
      await page.mouse.down();
      await page.waitForTimeout(100);
      await page.mouse.up();
      await waitFor(page, log, "Fleets' alerts turn off", () => flips().length === 1, 5_000);
      assert.match(flips()[0], /mask=0x80 silent=true/);
      await click(page, faction.secondDroid, "right");
      await page.waitForTimeout(400);
      await openSubmenu();
      await shot("alerts-submenu-fleets-off");
      const after = await checkCells();
      alerts = { before, after, flips: flips() };
      assert.deepEqual(before, Array(9).fill(true), JSON.stringify(alerts));
      assert.deepEqual(after, [true, false, true, true, true, true, true, true, true], JSON.stringify(alerts));
    }
    await page.keyboard.press("Escape");
    assert.deepEqual(errors, [], errors.join("; "));
    assert.ok(network.every(({ status }) => status < 400),
      JSON.stringify(network.filter(({ status }) => status >= 400)));
    return { faction: faction.name, status: "pass", posted: posted(), shots, rowsText, toggle, rail, textBar, alerts,
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
